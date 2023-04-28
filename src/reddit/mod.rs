mod scrape;
pub use scrape::*;

mod analyze;
pub use analyze::*;


use serde::{Deserialize, Serialize};
use std::{fs::{write, read_to_string}, path::Path};

pub trait Data {
    fn save(&self, file: &Path);
    fn restore(&mut self, file: &Path);
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Subreddit {
    /// The subreddit's name, such as "r/rust".
    pub name: String,
    /// A brief description of the subreddit provided by the moderators.
    pub description: String,
    /// The number of subscribers to the subreddit.
    pub subscribers: u64,
    /// The posts on the subreddit.
    pub posts: Vec<Post>
}

impl Data for Subreddit {
    fn save(&self, file: &Path) {
        if let Ok(output_json) = serde_json::to_string(&self) {
            write(file, output_json).unwrap();
        }
    }

    fn restore(&mut self, file: &Path) {
        if let Ok(input_json) = read_to_string(file) {
            *self = serde_json::from_str(&input_json).unwrap();
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Post {
    /// The title of the post.
    pub title: String,
    /// Is this post not safe for work?
    pub not_safe_for_work: bool,
    /// Is this post locked by the moderators?
    pub locked: bool,
    /// The post content.
    pub body: String,
    /// The score (upvotes - downvotes) the post has received.
    pub score: i32,
    /// The comments on the post.
    pub comments: Vec<Comment>
}

impl Data for Post {
    fn save(&self, file: &Path) {
        if let Ok(output_json) = serde_json::to_string(&self) {
            write(file, output_json).unwrap();
        }
    }

    fn restore(&mut self, file: &Path) {
        if let Ok(input_json) = read_to_string(file) {
            *self = serde_json::from_str(&input_json).unwrap();
        }
    }
}


#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Comment {
    /// The text of the comment.
    pub body: String,
    /// The score (upvotes - downvotes) the comment has received.
    pub score: i32,
    /// The replies to the comment.
    pub comments: Vec<Comment>
}

impl Data for Comment {
    fn save(&self, file: &Path) {
        if let Ok(output_json) = serde_json::to_string(&self) {
            write(file, output_json).unwrap();
        }
    }

    fn restore(&mut self, file: &Path) {
        if let Ok(input_json) = read_to_string(file) {
            *self = serde_json::from_str(&input_json).unwrap();
        }
    }
}
