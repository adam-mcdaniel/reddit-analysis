use rust_bert::{
    pipelines::{
        zero_shot_classification::{ZeroShotClassificationModel, ZeroShotClassificationConfig},
    }
};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};
use std::{sync::Mutex, thread::sleep, time::Duration};

const MAX_LENGTH: usize = 192;

#[derive(Clone, Debug)]
pub enum AnalysisError {
    SentimentError(String),
    ZeroShotError(String),
    LabelError(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Attitude {
    Inquisitive,
    Praise,
    Condemnation,
    Agreement,
    Complaint,
    Mocking,
    Disagreement,
    Annoyed,
    Neutral
}

impl Attitude {
    const TRESHOLD: f64 = 0.3;

    pub const VALUES: [Self; 9] = [
        Self::Neutral,
        Self::Inquisitive,
        Self::Praise,
        Self::Condemnation,
        Self::Agreement,
        Self::Complaint,
        Self::Mocking,
        Self::Disagreement,
        Self::Annoyed,
    ];

    const LABELS: [&'static str; 9] = [
        "question",
        "praise",
        "condemnation",
        "agreement",
        "complaint",
        "mocking",
        "disagreement",
        "annoyed",
        "neutral"
    ];

    fn from_label(label: &str) -> Result<Self, AnalysisError> {
        Ok(match label {
            "question" => Self::Inquisitive,
            "praise" => Self::Praise,
            "agreement" => Self::Agreement,
            "complaint" => Self::Complaint,
            "mocking" => Self::Mocking,
            "disagreement" => Self::Disagreement,
            "annoyed" => Self::Annoyed,
            "condemnation" => Self::Condemnation,
            "neutral" => Self::Neutral,
            _ => return Err(AnalysisError::LabelError(format!("unknown label: {}", label))),
        })
    }

    /// How much the given label is associated with a positive attitude.
    pub fn positivity(&self) -> f64 {
        match self {
            Self::Inquisitive => 0.5,
            Self::Praise => 1.0,
            Self::Agreement => 0.8,
            Self::Complaint => 0.0,
            Self::Mocking => 0.0,
            Self::Disagreement => 0.25,
            Self::Annoyed => 0.0,
            Self::Neutral => 0.5,
            Self::Condemnation => 0.0,
        }
    }

    /// How much the given label is associated with a negative attitude.
    pub fn negativity(&self) -> f64 {
        1.0 - self.positivity()
    }

    /// How much the given label is associated with agreement?
    pub fn agreement(&self) -> f64 {
        match self {
            Self::Inquisitive => 0.5,
            Self::Praise => 0.8,
            Self::Agreement => 1.0,
            Self::Complaint => 0.3,
            Self::Mocking => 0.5,
            Self::Disagreement => 0.0,
            Self::Annoyed => 0.2,
            Self::Neutral => 0.5,
            Self::Condemnation => 0.0,
        }
    }
}

impl ToString for Attitude {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subject {
    Politics,
    Religion,
    Science,
    Food,
    Animals,
    Sports,
    Music,
    Movies,
    Joke,
    Technology,
    Discussion,
    Personal,
    Other
}

impl Subject {
    const TRESHOLD: f64 = 0.3;

    pub const VALUES: [Self; 13] = [
        Self::Other,
        Self::Politics,
        Self::Religion,
        Self::Science,
        Self::Food,
        Self::Animals,
        Self::Sports,
        Self::Music,
        Self::Movies,
        Self::Joke,
        Self::Technology,
        Self::Discussion,
        Self::Personal,
    ];

    const LABELS: [&'static str; 12] = [
        "politics",
        "religion",
        "science",
        "food",
        "animals",
        "sports",
        "music",
        "movies",
        "joke",
        "technology",
        "discussion",
        "me",
    ];

    fn from_label(label: &str) -> Result<Self, AnalysisError> {
        Ok(match label {
            "politics" => Self::Politics,
            "religion" => Self::Religion,
            "science" => Self::Science,
            "food" => Self::Food,
            "animals" => Self::Animals,
            "sports" => Self::Sports,
            "music" => Self::Music,
            "movies" => Self::Movies,
            "joke" => Self::Joke,
            "technology" => Self::Technology,
            "discussion" => Self::Discussion,
            "me" => Self::Personal,
            _ => return Err(AnalysisError::LabelError(format!("unknown label: {}", label))),
        })
    }
}

impl ToString for Subject {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Analysis {
    // How positive or negative is the text?
    // pub sentiment: bool,
    // What is the attitude of the text?
    pub attitude: Attitude,
    /// How confident is the model in its attitude analysis?
    pub attitude_confidence: f64,
    // What is the subject of the text?
    pub subject: Subject,
    /// How confident is the model in its subject analysis?
    pub subject_confidence: f64,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            // sentiment: false,
            attitude: Attitude::Neutral,
            attitude_confidence: 0.0,
            subject: Subject::Other,
            subject_confidence: 0.0,
        }
    }
}

lazy_static! {
    // static ref SENTIMENT_MODEL: Mutex<SentimentModel> = {
    //     e// println!("loading sentiment model...");
    //     let result = Mutex::new(SentimentModel::new(Default::default()).expect("failed to load sentiment model"));
    //     e// println!("done loading sentiment model");
    //     result
    // };
    static ref ZERO_SHOT_MODEL_0: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 0...");
        // config.model_type = ModelType::Roberta;
        let config: ZeroShotClassificationConfig = Default::default();
        // config.model_type = ModelType::DistilBert;
        // config.model_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertModelResources::DISTIL_BERT_SST2,
        //     // gpt2::Gpt2ModelResources::DISTIL_GPT2,
        //     // roberta::RobertaModelResources::ALL_DISTILROBERTA_V1,
        // ));
        // config.config_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertConfigResources::DISTIL_BERT_SST2,
        //     // gpt2::Gpt2ConfigResources::DISTIL_GPT2,
        //     // roberta::RobertaConfigResources::ALL_DISTILROBERTA_V1,
        // ));
        // config.vocab_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertVocabResources::DISTIL_BERT_SST2,
        //     // gpt2::Gpt2VocabResources::DISTIL_GPT2,
        //     // roberta::RobertaVocabResources::ALL_DISTILROBERTA_V1,
        // ));
        // config.merges_resource = Some(Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertVocabResources::DISTIL_BERT_SST2,
        //     // gpt2::Gpt2MergesResources::DISTIL_GPT2,
        //     // roberta::RobertaMergesResources::ALL_DISTILROBERTA_V1,
        // )));
        // config.merges_resource = None;
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 0");
        result
    };
    static ref ZERO_SHOT_MODEL_1: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 1...");
        let config: ZeroShotClassificationConfig = Default::default();
        // config.model_type = ModelType::DistilBert;
        // config.model_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertModelResources::DISTIL_BERT_SST2,
        //     // albert::AlbertModelResources::ALBERT_BASE_V2,
        //     // roberta::RobertaModelResources::DISTILROBERTA_BASE,
        // ));
        // config.config_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertConfigResources::DISTIL_BERT_SST2,
        //     // albert::AlbertConfigResources::ALBERT_BASE_V2,
        //     // roberta::RobertaConfigResources::DISTILROBERTA_BASE,
        // ));
        // config.vocab_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertVocabResources::DISTIL_BERT_SST2,
        //     // albert::AlbertVocabResources::ALBERT_BASE_V2,
        //     // roberta::RobertaVocabResources::DISTILROBERTA_BASE,
        // ));
        // config.merges_resource = None;
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 1");
        result
    };
    static ref ZERO_SHOT_MODEL_2: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 2...");
        let config: ZeroShotClassificationConfig = Default::default();
        // config.model_type = ModelType::DistilBert;
        // config.model_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertModelResources::DISTIL_BERT_SST2,
        //     // albert::AlbertModelResources::ALBERT_BASE_V2,
        //     // roberta::RobertaModelResources::DISTILROBERTA_BASE,
        // ));
        // config.config_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertConfigResources::DISTIL_BERT_SST2,
        //     // albert::AlbertConfigResources::ALBERT_BASE_V2,
        //     // roberta::RobertaConfigResources::DISTILROBERTA_BASE,
        // ));
        // config.vocab_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertVocabResources::DISTIL_BERT_SST2,
        //     // albert::AlbertVocabResources::ALBERT_BASE_V2,
        //     // roberta::RobertaVocabResources::DISTILROBERTA_BASE,
        // ));
        // config.merges_resource = None;
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 2");
        result
    };
    static ref ZERO_SHOT_MODEL_3: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 3...");
        let config: ZeroShotClassificationConfig = Default::default();
        // config.model_type = ModelType::DistilBert;
        // config.model_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertModelResources::DISTIL_BERT_SST2,
        //     // albert::AlbertModelResources::ALBERT_BASE_V2,
        //     // roberta::RobertaModelResources::DISTILROBERTA_BASE,
        // ));
        // config.config_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertConfigResources::DISTIL_BERT_SST2,
        //     // albert::AlbertConfigResources::ALBERT_BASE_V2,
        //     // roberta::RobertaConfigResources::DISTILROBERTA_BASE,
        // ));
        // config.vocab_resource = Box::new(RemoteResource::from_pretrained(
        //     distilbert::DistilBertVocabResources::DISTIL_BERT_SST2,
        //     // albert::AlbertVocabResources::ALBERT_BASE_V2,
        //     // roberta::RobertaVocabResources::DISTILROBERTA_BASE,
        // ));
        // config.merges_resource = None;
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 3");
        result
    };
    static ref ZERO_SHOT_MODEL_4: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 4...");
        let config: ZeroShotClassificationConfig = Default::default();
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 4");
        result
    };
    static ref ZERO_SHOT_MODEL_5: Mutex<ZeroShotClassificationModel> = {
        // println!("Loading model 5...");
        let config: ZeroShotClassificationConfig = Default::default();
        let result = Mutex::new(ZeroShotClassificationModel::new(config).expect("failed to load zero shot model"));
        // println!("Loaded model 5");
        result
    };
}

pub trait Analyze {
    fn analyze(&self) -> Result<Analysis, AnalysisError>;
}

impl Analyze for &str {
    fn analyze(&self) -> Result<Analysis, AnalysisError> {
        // println!("Analyzing: '{}'", self);
        if self == &"" {
            return Ok(Analysis::default());
        }
        let input = [*self];
        let pool: &[&Mutex<ZeroShotClassificationModel>] = &[
            &ZERO_SHOT_MODEL_0,
            &ZERO_SHOT_MODEL_1,
            &ZERO_SHOT_MODEL_2,
            &ZERO_SHOT_MODEL_3,
            // &ZERO_SHOT_MODEL_4,
//            &ZERO_SHOT_MODEL_5
        ];
        // let sentiment_model = SENTIMENT_MODEL.lock().unwrap();
        let mut i = 0;
        loop {
            if let Ok(ref mut classify_model) = pool[i].try_lock() {
                // println!("Acquired {i}");
                // let output = classify_model.predict(&input, &Subject::LABELS).unwrap();
                // let classify_model = ZERO_SHOT_MODEL.lock().unwrap();
        
                // let output = sentiment_model.predict(&input);
                let mut subjects = classify_model.predict(
                    &input,
                    &Subject::LABELS,
                    // None,
                    Some(Box::new(|label| format!("This text's subject is {}", label))),
                    MAX_LENGTH,
                );
                subjects.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
                let mut attitudes = classify_model.predict(
                    &input,
                    &Attitude::LABELS,
                    Some(Box::new(|label| format!("This text's attitude is {}", label))),
                    // None,
                    MAX_LENGTH,
                );
                attitudes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                // let sentiment = output[0].polarity == SentimentPolarity::Positive;
                let (attitude, attitude_confidence) = (attitudes[0].score > Attitude::TRESHOLD)
                    .then_some((Attitude::from_label(&attitudes[0].text)?, attitudes[0].score))
                    .unwrap_or((Attitude::Neutral, 1.0 - attitudes[0].score));
                let (subject, subject_confidence) = (subjects[0].score > Subject::TRESHOLD)
                    .then_some((Subject::from_label(&subjects[0].text)?, subjects[0].score))
                    .unwrap_or((Subject::Other, 1.0 - subjects[0].score));
                let result = Analysis {
                    // sentiment,
                    attitude,
                    attitude_confidence,
                    subject,
                    subject_confidence,
                };
                // println!("Result: '{}'\n -> {result:#?}", self);
                return Ok(result)
            }
            i += 1;
            i = i % pool.len();
            sleep(Duration::from_millis(100));
        }
    }
}

