use roux::{Subreddit, reply::MaybeReplies};
use indicatif::{ProgressBar, MultiProgress, ProgressStyle};
use std::{thread::sleep, time::Duration, path::PathBuf};
use super::Data;

#[derive(Clone, Debug)]
pub enum ScrapeError {
    RedditError(String),
    CouldNotRead(String)
}

impl<E> From<E> for ScrapeError where E: std::error::Error {
    fn from(err: E) -> Self {
        Self::RedditError(format!("{err:?}"))
    }
}


pub trait Scrape<T> {
    fn scrape(info: T) -> Result<Box<Self>, ScrapeError>;
}

impl Scrape<&str> for super::Subreddit {
    fn scrape(subreddit: &str) -> Result<Box<Self>, ScrapeError> {
        Self::scrape(Subreddit::new(subreddit))
    }
}

impl Scrape<roux::comment::CommentData> for super::Comment {
    fn scrape(comment: roux::comment::CommentData) -> Result<Box<Self>, ScrapeError> {
        Ok(Box::new(Self {
            body: comment.body.unwrap_or(String::new()),
            score: comment.score.unwrap_or(0),
            comments: match comment.replies {
                Some(MaybeReplies::Reply(raw_replies)) => {
                    let mut replies = vec![];
                    for reply in raw_replies.data.children {
                        replies.push(*Self::scrape(reply.data)?);
                    }
                    replies
                }
                _ => vec![]
            }
        }))
    }
}

impl Scrape<roux::submission::SubmissionData> for super::Post {
    fn scrape(post: roux::submission::SubmissionData) -> Result<Box<Self>, ScrapeError> {
        let subreddit = Subreddit::new(&post.subreddit);
        Ok(Box::new(Self {
            title: post.title,
            not_safe_for_work: post.over_18,
            locked: post.locked,
            body: post.selftext,
            score: post.score as i32,
            comments: {
                let raw_comments = subreddit.article_comments(&post.id, None, Some(15))?;

                let mut comments = vec![];
                for comment in raw_comments.data.children {
                    comments.push(*super::Comment::scrape(comment.data)?);
                }
                comments
            }
        }))
    }
}

const WAIT_BETWEEN_POSTS_SECONDS: f64 = 0.0;
const WAIT_BETWEEN_SUBREDDITS_SECONDS: f64 = 3.0;
const POSTS_PER_SUBREDDIT: u32 = 100;
const COMMENTS_PER_POST: u32 = 10;

impl Scrape<roux::Subreddit> for super::Subreddit {
    fn scrape(subreddit: roux::Subreddit) -> Result<Box<Self>, ScrapeError> {
        let name = subreddit.name.clone();
        let results = *Vec::scrape(&[subreddit.name])?;
        match results.into_iter().next() {
            Some(val) => Ok(Box::new(val)),
            None => Err(ScrapeError::CouldNotRead(name))
        }
    }
}


impl<T> Scrape<&[T]> for Vec<super::Subreddit> where T: AsRef<str> {
    fn scrape(subreddits: &[T]) -> Result<Box<Self>, ScrapeError> {
        let subreddit_style = ProgressStyle::with_template(
            " [{elapsed_precise}] {prefix:<22} {bar:30.cyan/red} {pos}/{len} {msg} {spinner}",
        )?;
        let submission_style = ProgressStyle::with_template(
            " [{elapsed_precise}] {prefix:<22} {bar:30.cyan/red} {percent:>3}% [{per_sec:10}] {msg} {spinner}",
        )?;

        let multi_bar = MultiProgress::new();
        let subreddit_bar = multi_bar.add(ProgressBar::new(subreddits.len() as u64));
        subreddit_bar.set_style(subreddit_style);
        subreddit_bar.set_prefix("Collecting subreddits");
        let post_bar = multi_bar.insert_after(&subreddit_bar, ProgressBar::new(100));
        post_bar.set_style(submission_style.clone());
        post_bar.set_prefix("Collecting posts");
        let comment_bar = multi_bar.insert_after(&post_bar, ProgressBar::new(100));
        comment_bar.set_style(submission_style);
        comment_bar.set_prefix("Collecting comments");
        
        
        let mut result = vec![];
        for subreddit_name in subreddits {
            let output_path = format!("./data/{}.json", subreddit_name.as_ref());

            if PathBuf::from(&output_path).exists() {
                subreddit_bar.set_message(format!("Already have data for {}, skipping", subreddit_name.as_ref()));
                subreddit_bar.inc(1);
                continue;
            }

            subreddit_bar.set_message(format!("r/{}", subreddit_name.as_ref()));
            
            post_bar.reset();
            comment_bar.reset();
            
            post_bar.set_message("Retrieving post IDs...");
            
            
            let subreddit = Subreddit::new(subreddit_name.as_ref());
            let raw_posts = match subreddit.hot(POSTS_PER_SUBREDDIT, None) {
                Ok(posts) => posts,
                Err(_) => {
                    post_bar.set_message("Error retrieving post");
                    sleep(Duration::from_millis(500));
                    continue
                }
            };
            let total_post_count = raw_posts.data.children.len() as u64;
            let mut posts = vec![];

            post_bar.set_message(format!("Retrieved {} post IDs", total_post_count));
            post_bar.reset();
            post_bar.set_length(total_post_count);

            let mut raw_comments = vec![];
            for (i, raw_post) in raw_posts.data.children.iter().enumerate() {
                post_bar.set_message(format!("Retrieving post comments {i}/{total_post_count}"));
                subreddit_bar.tick();
                comment_bar.tick();
                raw_comments.push(subreddit.article_comments(&raw_post.data.id, None, Some(COMMENTS_PER_POST)));
                post_bar.inc(1);
            }
            post_bar.reset();
            for (i, (raw_post, raw_comments)) in raw_posts.data.children.iter().zip(raw_comments).enumerate() {
                let post = &raw_post.data;
                for i in 0..(WAIT_BETWEEN_POSTS_SECONDS * 10.0) as usize {
                    post_bar.set_message(format!("Waiting {}/{}s for rate limit...", i/10, WAIT_BETWEEN_POSTS_SECONDS));
                    subreddit_bar.tick();
                    post_bar.tick();
                    comment_bar.tick();
                    sleep(Duration::from_millis(100));
                }
                post_bar.set_message(format!("Awaiting post {i}/{total_post_count} comments..."));
                let raw_comments = match raw_comments {
                    Ok(c) => c,
                    Err(_) => {
                        post_bar.set_message("Error retrieving post");
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                };
                post_bar.set_message(format!("Retrieved post {i}/{total_post_count} comments"));
                posts.push(super::Post {
                    title: post.title.clone(),
                    not_safe_for_work: post.over_18,
                    locked: post.locked,
                    body: post.selftext.clone(),
                    score: post.score as i32,
                    comments: {
                        if raw_comments.data.children.is_empty() {
                            vec![]
                        } else {
                            comment_bar.reset();
                            comment_bar.set_length(raw_comments.data.children.len() as u64);
                            
                            comment_bar.set_message(format!("Processing {} replies...", raw_comments.data.children.len()));
                            let mut comments = vec![];
                            for comment in raw_comments.data.children {
    
                                comment_bar.set_message("Retrieving replies...");
                                comments.push(*super::Comment::scrape(comment.data)?);
                                comment_bar.set_message("Retrieved replies");
                                
                                subreddit_bar.tick();
                                post_bar.tick();
                                comment_bar.inc(1);
                            }
                            comments
                        }
                    }
                });
                comment_bar.set_message("Done with post replies");
                post_bar.set_message(format!("Finished post {i}/{}", total_post_count));
                post_bar.inc(1);
            }
            post_bar.set_message("Finished subreddit posts");

            subreddit_bar.set_message("Processing subreddit...");
            let about = subreddit.about();
            let subreddit = super::Subreddit {
                name: subreddit.name.clone(),
                description: match &about {
                    Ok(s) => s.public_description.clone().unwrap_or("".to_string()),
                    Err(_) => "".to_string()
                },
                subscribers: match about {
                    Ok(s) => s.subscribers.unwrap_or(0),
                    Err(_) => 0
                },
                posts
            };
            subreddit_bar.set_message(format!("Saving to {output_path}..."));
            subreddit.save(&PathBuf::from(output_path.clone()));
            result.push(subreddit);
            subreddit_bar.set_message(format!("Saved to {output_path}"));
            subreddit_bar.inc(1);

            for i in 0..(WAIT_BETWEEN_SUBREDDITS_SECONDS * 10.0) as usize {
                subreddit_bar.set_message(format!("Waiting {}/{}s for rate limit...", i/10, WAIT_BETWEEN_SUBREDDITS_SECONDS));
                subreddit_bar.tick();
                post_bar.tick();
                comment_bar.tick();
                sleep(Duration::from_millis(100));
            }
        }
        subreddit_bar.set_message("Finished");
        post_bar.set_message("Finished");
        comment_bar.set_message("Finished");
        post_bar.finish_and_clear();
        comment_bar.finish_and_clear();
        subreddit_bar.finish_and_clear();

        multi_bar.clear()?;
        
        Ok(Box::new(result))
    }
}
