/**
 * @file levenshtein.rs
 * @author Krisna Pranav
 * @version 0.1
 * @date 2023-05-11
 *
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoBlocksDevelopers
 *
*/


use std::cmp;

pub fn distance(source: &str, target: &str) -> usize {
    if source.is_empty() {
        return target.len();
    }
    if target.is_empty() {
        return source.len();
    }

    let mut distances = (0..=target.chars().count()).collect::<Vec<_>>();

    for (i, ch1) in source.chars().enumerate() {
        let mut sub = i;
        distances[0] = sub + 1;
        for (j, ch2) in target.chars().enumerate() {
            let dist = cmp::min(
                cmp::min(distances[j], distances[j + 1]) + 1,
                sub + (ch1 != ch2) as usize,
            );

            sub = distances[j + 1];
            distances[j + 1] = dist;
        }
    }

    *distances.last().unwrap()
}