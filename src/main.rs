extern crate argparse;

use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::BuildHasherDefault;
use std::fs::File;
use std::path::Path;
use std::io::{self, prelude::*};

fn subtract(word: &[u8], pool: &[u8]) -> Option<Box<[u8]>> {
    let mut i = 0;
    let mut result = vec![];

    for &c in pool {
        if i < word.len() && word[i] == c {
            i += 1;
        } else {
            result.push(c);
        }
    }

    if i >= word.len() {
        return Some(result.into_boxed_slice())
    }
    None
}

fn make_key(word: &str) -> Option<Box<[u8]>> {
    let mut bs: Vec<u8> = word.into();
    if bs.iter().any(|&i| i >= 0x80) { return None }

    // Take only the alphabetic characters
    let mut i = 0;
    loop {
        if i >= bs.len() { break }
        match bs[i] {
            b'A' ..= b'Z' => {
                bs[i] += 0x20;
                i += 1;
            },
            b'a' ..= b'z' | b'0' ..= b'9' => {
                i += 1;
            },
            _ => {
                bs.swap_remove(i);
            },
        }
    }

    bs.sort();
    Some(bs.into_boxed_slice())
}

type Dictionary = HashMap<Box<[u8]>, Vec<Box<str>>, BuildHasherDefault<DefaultHasher>>;
type Iter<'a> = std::collections::hash_map::Iter<'a, Box<[u8]>, Vec<Box<str>>>;

pub struct Anagrammer {
    dictionary: Dictionary,
}

impl Anagrammer {
    pub fn from_dictionary_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut dictionary = Dictionary::default();

        for line in contents.split('\n') {
            let word = line.trim();
            if word.len() == 0 { continue }

            if let Some(bs) = make_key(&word) {
                dictionary.entry(bs).or_insert_with(|| vec![]).push(word.into());
            }
        }

        Ok(Anagrammer {
            dictionary,
        })
    }

    pub fn from_default_list() -> Self {
        let mut dictionary = Dictionary::default();

        for line in include_str!("english-words").split('\n') {
            let w = line.trim().to_lowercase();
            if w.len() == 0 { continue }
            
            if let Some(bs) = make_key(&w) {
                dictionary.entry(bs).or_insert_with(|| vec![]).push(w.into_boxed_str());
            }
        }

        Anagrammer {
            dictionary,
        }
    }

    fn restrict(&mut self, pool: &[u8]) {
        self.dictionary.retain(|key, _| {
            subtract(key, pool).is_some()
        });
    }

    pub fn restrict_letters(&mut self, minletters: usize, maxletters: usize) {
        self.dictionary.retain(|key, _| {
            key.len() >= minletters && key.len() <= maxletters
        });
    }

    pub fn find_anagrams<F: FnMut(Vec<&str>)>(mut self, word: &[u8], minwords: usize, maxwords: usize, mut f: F) {
        let mut pool = word.iter().cloned().collect::<Vec<_>>();
        pool.sort();
        self.restrict(&pool);
        self.anagrams_recur(self.dictionary.iter(), &pool, minwords, maxwords, &mut f);
    }

    fn anagrams_recur(&self, mut dictionary_iter: Iter, pool: &[u8], minwords: usize, maxwords: usize, f: &mut dyn FnMut(Vec<&str>)) {
        if minwords > maxwords {
            return
        }

        if pool.len() == 0 && minwords == 0 {
            f(vec![]);
            return
        }

        if maxwords == 0 {
            return
        }

        if maxwords == 1 {
            if let Some((key, words)) = self.dictionary.get_key_value(pool) {
                let opt = dictionary_iter.next();
                if opt.is_none() { return }
                let (next_key, _) = opt.unwrap();

                // Make sure to skip any words that we've already searched
                // to avoid permutations of the same anagram
                if next_key as *const _ > key as *const _ { return }

                for word in words {
                    f(vec![word]);
                }
            }
            return
        }

        while let Some((key, words)) = dictionary_iter.next() {
            if let Some(new_pool) = subtract(key, pool) {
                let new_minwords = if minwords == 0 { 0 } else { minwords - 1 };
                self.anagrams_recur(dictionary_iter.clone(), &new_pool, new_minwords, maxwords - 1, &mut |set| {
                    for word in words {
                        let mut new_set = set.clone();
                        new_set.push(word);
                        f(new_set);
                    }
                })
            }
        }
    }
}

fn print_set(set: Vec<&str>) {
    let mut first = true;
    for item in set.iter().rev() {
        if !first {
            print!(" ");
        }
        print!("{}", item);
        first = false;
    }
    println!("");
}

fn main() -> std::io::Result<()> {
    use argparse::{ArgumentParser, Store};

    let (mut minwords, mut maxwords) = (0, std::usize::MAX);
    let (mut minletters, mut maxletters) = (0, std::usize::MAX);

    let mut dictionary_path = String::new();

    let mut string = String::new();

    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Find anagrams of the given string");
        ap.refer(&mut string)
            .required()
            .add_argument("string", Store, "String to generate anagrams of");
        ap.refer(&mut minwords)
            .add_option(&["-w", "--min-words"], Store, "The minimum number of words in the generated anagrams");
        ap.refer(&mut maxwords)
            .add_option(&["-W", "--max-words"], Store, "The maximum number of words in the generated anagrams");
        ap.refer(&mut minletters)
            .add_option(&["-l", "--min-words"], Store, "The minimum number of letters per word in the generated anagrams");
        ap.refer(&mut maxletters)
            .add_option(&["-L", "--max-words"], Store, "The maximum number of letters per word in the generated anagrams");
        ap.refer(&mut dictionary_path)
            .add_option(&["-f", "--dictionary"], Store, "The path of the word list");
        ap.parse_args_or_exit();
    }

    let bytes = make_key(&string).unwrap_or_else(|| {
        eprintln!("error: only ASCII strings are supported");
        std::process::exit(1)
    });

    let mut anagrammer = if dictionary_path.len() == 0 {
        Anagrammer::from_default_list()
    } else {
        Anagrammer::from_dictionary_path(&dictionary_path)?
    };

    if (minletters, maxletters) != (0, std::usize::MAX) {
        anagrammer.restrict_letters(minletters, maxletters);
    }

    anagrammer.find_anagrams(&bytes, minwords, maxwords, &mut print_set);
    Ok(())
}
