use std::{path::Path, sync::Arc, thread::{self, sleep}, time::Duration};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
// use std::path::Path;
use reddit_analyzer::*;
use clap::Parser;

const MIN_POST_SIZE: usize = 3;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
   /// Name of subreddit to analyze
   subreddit: String,
}

fn main() {
    let args = Args::parse();
    
    let subreddit_name = args.subreddit;
    let mut sub = Subreddit::default();
    sub.restore(Path::new(&format!("data/{subreddit_name}.json")));
    sub.posts = sub.posts.into_iter().filter(|post| post.size() >= MIN_POST_SIZE).collect();
    let mut post_analyses = vec![];

    let multi_bar = MultiProgress::new();
    let post_bar = multi_bar.add(ProgressBar::new(100));

    post_bar.set_prefix("Analyzing posts...");
    post_bar.set_style(ProgressStyle::with_template(
        " [{elapsed_precise}] {prefix:<22} {bar:30.cyan/red} {pos:>3}/{len:<3} [{eta_precise:8}] {msg} {spinner}",
    ).unwrap());
    post_bar.set_message("Reading posts...");


    let total_posts = sub.posts.len();
    post_bar.set_length(total_posts as u64);
    post_bar.enable_steady_tick(Duration::from_millis(100));
    let mut total_size = 0;
    for post in &sub.posts {
        let size = post.size();
        total_size += size;
    }
    post_bar.set_message("Posts read");

    post_bar.set_prefix(format!("Analyzing {} posts", sub.posts.len()));
    let comment_bar_style = ProgressStyle::with_template(
        " [{elapsed_precise}] {prefix:<22} {bar:30.cyan/red} {pos:>3}/{len:<3} [{per_sec:8}] {msg} {spinner}",
    ).unwrap();
    let mut is_first = true;
    for (i, post) in sub.posts.iter().enumerate() {
        post_bar.set_message(format!("Analyzing post #{}", i + 1));
        let post_size = post.size();
        let comment_bar = Arc::new(multi_bar.insert_after(&post_bar, ProgressBar::new(post_size as u64)));
        comment_bar.set_prefix(format!("Analyzing {total_size} comments"));
        if is_first {
            comment_bar.set_message("Loading models...");
            is_first = false;
        } else {
            comment_bar.set_message("Analyzing post comments...");
        }

        comment_bar.set_style(comment_bar_style.clone());
        let comment_bar_clone = comment_bar.clone();
        let handle = thread::spawn(move || {
            loop {
                let total_analyzed = ANALYZED_COMMENTS.lock().unwrap();
                if *total_analyzed != 0 {
                    comment_bar_clone.set_message("Analyzing post comments...");
                }
                if *total_analyzed >= post_size {
                    break;
                }
                comment_bar_clone.set_position(*total_analyzed as u64);
                drop(total_analyzed);
                sleep(Duration::from_millis(100));
            }
            comment_bar_clone.finish_and_clear();
        });

        comment_bar.set_length(post.size() as u64);
        
        let analysis = post.analyze_submission().unwrap();
        comment_bar.set_message("Saving analysis...");
        analysis.save(Path::new(&format!("analysis/{subreddit_name}_post_analysis_{i}.json")));

        comment_bar.set_message("Cleaning up...");
        
        post_analyses.push(analysis);
        post_bar.inc(1);
        comment_bar.set_message("Joining threads...");
        handle.join().unwrap();

        let mut total_analyzed = ANALYZED_COMMENTS.lock().unwrap();
        *total_analyzed = 0;
        drop(total_analyzed);
    }
    post_bar.finish_and_clear();

    post_analyses.save(Path::new(&format!("analysis/{subreddit_name}_subreddit_analysis.json")));
}
