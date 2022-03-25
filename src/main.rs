use std::{
    fs,
    io::{Write, BufReader, BufRead},
    sync::{
        mpsc,
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use color_eyre::{
    Result,
};

use rand::prelude::*;

type Word = [u8; 5];

fn ord(c: u8) -> usize { (c - b'a') as usize }
fn score(covered: &[[bool; 26]; 5], word: Word) -> i32 {
    (0..5).filter(|&p| !covered[p][ord(word[p])]).count() as i32
}

fn generate_guesses(dict: &[Word], best_size: usize) -> Option<Vec<(Word, i32)>> {
    let mut covered = [[false; 26]; 5];
    let mut guesses = Vec::new();
    let mut rng = rand::thread_rng();

    let mut letters: [u8; 26] =
        (b'a'..=b'z').collect::<Vec<u8>>().try_into().unwrap();

    let mut poses = [0, 1, 2, 3, 4];

    poses.shuffle(&mut rng);
    for pos in poses {
        letters.shuffle(&mut rng);
        for letter in letters {
            if covered[pos][ord(letter)] {
                continue;
            }

            let options = dict.iter().filter(|w| w[pos] == letter);
            let guess = *options.max_by_key(|w| score(&covered, **w))
                .unwrap();
            guesses.push((guess, score(&covered, guess)));
            if guesses.len() >= best_size {
                return None;
            }

            for p in 0..5 {
                covered[p][ord(guess[p])] = true;
            }
        }
    }

    if covered.iter().any(|w| w.iter().any(|c| !c)) {
        eprintln!("no solution??");
        return None;
    }

    Some(guesses)
}

fn main() -> Result<()> {
    let mut dict: Vec<Word> = Vec::new();
    let f = BufReader::new(fs::File::open("words")?);
    for line in f.lines() {
        let line = line?;
        let word: Word = line.as_bytes().try_into()?;
        dict.push(word);
    }

    let dict = Arc::new(dict);

    let mut best_guesses: Vec<(Word, i32)> = Vec::new();
    let (tx, rx) = mpsc::channel::<Vec<(Word, i32)>>();

    let best_size = Arc::new(AtomicUsize::new(usize::MAX));

    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..8 {
        let dict = dict.clone();
        let tx = tx.clone();
        let best_size = best_size.clone();
        let counter = counter.clone();
        thread::spawn(move || {
            loop {
                let best_size = best_size.load(Ordering::Relaxed);
                if let Some(guesses) = generate_guesses(&dict, best_size) {
                    tx.send(guesses);
                }

                let counter = counter.fetch_add(1, Ordering::Relaxed) + 1;
                if counter & 0xffff == 0 {
                    dbg!(counter);
                }
            }
        });
    }

    for guesses in rx.iter() {
        if best_guesses.len() != 0 && guesses.len() >= best_guesses.len() {
            continue;
        }

        best_size.store(guesses.len(), Ordering::Relaxed);
        println!("best size: {}", guesses.len());
        best_guesses = guesses;
        let mut f = fs::File::create("best")?;
        for (guess, score) in &best_guesses {
            f.write(guess)?;
            writeln!(f, "\t{score}")?;
        }
    }

    Ok(())
}
