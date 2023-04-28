use crate::*;
use serde::{Deserialize, Serialize};
use std::{path::Path, fs::{read_to_string, write}};
use rayon::prelude::*;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SubmissionAnalysis {
    /// The analysis of the submission's content.
    pub analysis: Analysis,
    /// The analysis of the submission's comments.
    pub children: Vec<SubmissionAnalysis>,
}

impl Data for SubmissionAnalysis {
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

impl Data for Vec<SubmissionAnalysis> {
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

impl SubmissionAnalysis {
    /// How positive are the replies to this post?
    pub fn reply_positivity(&self) -> f64 {
        self.children
            .iter()
            .map(|child| child.analysis.attitude.positivity() * child.analysis.attitude_confidence)
            .sum::<f64>()
            / self.children.len() as f64
    }
    
    /// How much do the replies agree to this post?
    pub fn reply_agreement(&self) -> f64 {
        self.children
            .iter()
            .map(|child| child.analysis.attitude.agreement() * child.analysis.attitude_confidence)
            .sum::<f64>()
            / self.children.len() as f64
    }

    /// How divisive is the submission?
    /// Divisiveness is measured by comparing the split of the replies to the submission.
    /// If the replies are split evenly, the divisiveness is 1.0.
    /// If the replies are all positive, the divisiveness is 0.0.
    /// If the replies are all negative, the divisiveness is 0.0.
    pub fn divisiveness(&self) -> f64 {
        let mut positive = 0;
        let mut negative = 0;
        for child in &self.children {
            if child.analysis.attitude.agreement() > 0.5 {
                positive += 1;
            } else if child.analysis.attitude.agreement() < 0.5 {
                negative += 1;
            }
        }
        let total = positive + negative;
        if total == 0 {
            0.0
        } else {
            1.0 - ((positive - negative) as f64).abs() / total as f64
        }
    }

    /// What is the average consensus of the replies?
    pub fn average_reply(&self) -> Analysis {
        use std::collections::BTreeMap;
        let mut subjects = BTreeMap::new();
        let mut attitudes = BTreeMap::new();

        for reply in &self.children {
            let subject = reply.analysis.subject;
            let attitude = reply.analysis.attitude;
            let subject_count = subjects.entry(subject).or_insert(0.0);
            let attitude_count = attitudes.entry(attitude).or_insert(0.0);
            *subject_count += reply.analysis.subject_confidence;
            *attitude_count += reply.analysis.attitude_confidence;
        }

        Analysis {
            attitude: attitudes
                .iter()
                .filter(|(attitude, _)| !matches!(attitude, Attitude::Neutral))
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(attitude, _)| *attitude)
                .unwrap_or(Attitude::Neutral),
            attitude_confidence: attitudes
                .iter()
                .filter(|(attitude, _)| !matches!(attitude, Attitude::Neutral))
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, count)| *count / self.children.len() as f64)
                .unwrap_or(0.0),
            subject: subjects
                .iter()
                .filter(|(subject, _)| !matches!(subject, Subject::Other))
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(subject, _)| *subject)
                .unwrap_or(Subject::Other),
            subject_confidence: subjects
                .iter()
                .filter(|(subject, _)| !matches!(subject, Subject::Other))
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, count)| *count / self.children.len() as f64)
                .unwrap_or(0.0),
        }
    }

    /// What is the size of the submission's reply tree (including this submission)?
    pub fn size(&self) -> usize {
        self.children.iter().map(|child| child.size()).sum::<usize>() + 1
    }
}

/// A trait representing a submission posted to a subreddit, or as a reply to a post or comment.
pub trait Submission {
    fn content(&self) -> &str;
    fn score(&self) -> i32;
    fn replies(&self) -> &[Comment];
    fn size(&self) -> usize {
        (self.content() != "") as usize + self.replies().iter().map(|reply| reply.size()).sum::<usize>()
    }
}

impl Submission for Post {
    fn content(&self) -> &str {
        &self.body
    }

    fn score(&self) -> i32 {
        self.score
    }

    fn replies(&self) -> &[Comment] {
        &self.comments
    }
}

impl Submission for Comment {
    fn content(&self) -> &str {
        &self.body
    }

    fn score(&self) -> i32 {
        self.score
    }

    fn replies(&self) -> &[Comment] {
        &&self.comments
    }
}

impl AnalyzeSubmission for Post {}
impl AnalyzeSubmission for Comment {}


use lazy_static::lazy_static;
lazy_static! {
    pub static ref ANALYZED_COMMENTS: std::sync::Mutex<usize> = std::sync::Mutex::new(0);
}

pub trait AnalyzeSubmission: Submission {
    fn analyze_submission(&self) -> Result<SubmissionAnalysis, AnalysisError> {
        let replies = self.replies().to_vec();
        let children = replies
            .into_par_iter()
            .filter(|reply| reply.content() != "")
            // .into_iter()
            .map(|reply| reply.analyze_submission())
            .filter(|x| x.is_ok())
            .collect::<Result<Vec<_>, _>>()?;
        let analysis = self.content().analyze()?;
        {
            let mut total = ANALYZED_COMMENTS.lock().unwrap();
            *total += 1;
            drop(total);
        }
        
        Ok(SubmissionAnalysis { analysis, children })
    }
}
