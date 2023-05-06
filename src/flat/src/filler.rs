/**
 * @file filler.rs
 * @author Krisna Pranav
 * @brief Config
 * @version 0.1
 * @date 2023-05-06
 * 
 * @copyright Copyright (c) 2023 Krisna Pranav, NanoDevelopers
 * 
 */


/**
 * @breif: Filler[Start, End]
 */
pub enum Filler {
    FillerStart(Box<Filler>),
    FillerEnd,
}

/**
 * @brief: Filler[length]
 */
impl Filler {
    pub fn length(&self) -> usize {
        match self {
            Filler::FillerStart(f) => f.length() + 1,
            Filler::FillerEnd => 1,
        }
    }
}