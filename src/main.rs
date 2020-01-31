extern crate argparse;

use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::BuildHasherDefault;
use std::fs::File;
use std::path::Path;
use std::io::{self, prelude::*};

/// `Identifier` is used to denote a normalized (i.e., lowercased) and sorted string.
type Identifier = str;
type Dictionary = HashMap<Box<Identifier>, Vec<Box<Identifier>>, BuildHasherDefault<DefaultHasher>>;
type Iter<'a> = std::collections::hash_map::Iter<'a, Box<Identifier>, Vec<Box<Identifier>>>;

fn subtract(word: &Identifier, pool: &Identifier) -> Option<Box<Identifier>> {
    let mut result = String::new();

    let mut word_chars = word.chars().peekable();
    for c in pool.chars() {
        let next = word_chars.peek();
        if next.is_some() && *next.unwrap() == c {
            word_chars.next();
        } else {
            result.push(c);
        }
    }

    if word_chars.peek().is_none() {
        return Some(result.into())
    }
    None
}

fn make_key(word: &str) -> Box<Identifier> {
    let mut bs: Vec<char> = vec![];

    // Take only the alphabetic characters
    for c in word.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                bs.push(lc);
            }
        }
    }

    bs.sort();
    let mut string = String::with_capacity(bs.len()); // lower bound on UTF-8 len
    for c in bs {
        string.push(c);
    }

    string.into()
}

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

            let bs = make_key(&word);
            dictionary.entry(bs).or_insert_with(|| vec![]).push(word.into());
        }

        Ok(Anagrammer {
            dictionary,
        })
    }

    pub fn from_default_list() -> Self {
        let mut dictionary = Dictionary::default();

        for line in include_str!("english-words").split('\n') {
            let word = line.trim();
            if word.len() == 0 { continue }
            
            let bs = make_key(&word);
            dictionary.entry(bs).or_insert_with(|| vec![]).push(word.into());
        }

        Anagrammer {
            dictionary,
        }
    }

    fn restrict(&mut self, pool: &Identifier) {
        self.dictionary.retain(|key, _| {
            subtract(key, pool).is_some()
        });
    }

    pub fn restrict_letters(&mut self, minletters: usize, maxletters: usize) {
        self.dictionary.retain(|key, _| {
            key.len() >= minletters && key.len() <= maxletters
        });
    }

    pub fn find_anagrams<F: FnMut(Vec<&str>)>(mut self, pool: &Identifier, minwords: usize, maxwords: usize, mut f: F) {
        self.restrict(pool);
        self.anagrams_recur(self.dictionary.iter(), pool, minwords, maxwords, &mut f);
    }

    fn anagrams_recur(&self, mut dictionary_iter: Iter, pool: &Identifier, minwords: usize, maxwords: usize, f: &mut dyn FnMut(Vec<&str>)) {
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

    let mut anagrammer = if dictionary_path.len() == 0 {
        Anagrammer::from_default_list()
    } else {
        Anagrammer::from_dictionary_path(&dictionary_path)?
    };

    if (minletters, maxletters) != (0, std::usize::MAX) {
        anagrammer.restrict_letters(minletters, maxletters);
    }

    let bytes = make_key(&string);

    anagrammer.find_anagrams(&bytes, minwords, maxwords, &mut print_set);
    Ok(())
}
