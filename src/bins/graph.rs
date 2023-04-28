use reddit_analyzer::*;
use std::collections::HashMap;
use plotters::{prelude::*, style::colors::full_palette::PURPLE, coord::Shift};

#[derive(Debug)]
struct SubredditData {
    subreddit: Subreddit,
    analysis: Vec<SubmissionAnalysis>,
}

fn traverse<T>(analysis: &SubmissionAnalysis, f: &impl Fn(&SubmissionAnalysis) -> T) -> Vec<T> {
    let mut result: Vec<T> = analysis.children.iter().map(|child| traverse(child, f)).flatten().collect();
    result.push(f(analysis));
    result
}

impl SubredditData {
    fn new(subreddit_name: &str) -> Self {
        let mut subreddit = Subreddit::default();
        subreddit.restore(std::path::Path::new(format!("data/{subreddit_name}.json").as_str()));

        let mut analysis: Vec<SubmissionAnalysis> = vec![];
        analysis.restore(std::path::Path::new(format!("analysis/{subreddit_name}_subreddit_analysis.json").as_str()));

        Self {
            subreddit,
            analysis,
        }
    }

    fn total_comments(&self) -> usize {
        self.analysis.iter().map(|a| a.size()).sum()
    }

    fn total_positive_comments(&self) -> usize {
        self.analysis
            .iter()
            .map(|submission| {
                traverse(submission, &|analysis| analysis.analysis.attitude.positivity() > 0.5)
                    .into_iter()    
                    .filter(|a| *a)
                    .count()
            })
            .sum()
    }

    fn total_negative_comments(&self) -> usize {
        self.analysis
            .iter()
            .map(|submission| {
                traverse(submission, &|analysis| analysis.analysis.attitude.negativity() > 0.5)
                    .into_iter()    
                    .filter(|a| *a)
                    .count()
            })
            .sum()
    }

    fn total_agreeability(&self) -> usize {
        self.analysis
            .iter()
            .map(|submission| {
                traverse(submission, &|analysis| analysis.analysis.attitude.agreement() > 0.5)
                    .into_iter()    
                    .filter(|a| *a)
                    .count()
            })
            .sum()
    }

    fn total_disagreeability(&self) -> usize {
        self.analysis
            .iter()
            .map(|submission| {
                traverse(submission, &|analysis| 1.0 - analysis.analysis.attitude.agreement() > 0.5)
                    .into_iter()    
                    .filter(|a| *a)
                    .count()
            })
            .sum()
    }

    fn total_divisiveness(&self) -> f64 {
        let agree = self.total_agreeability() as f64;
        let disagree = self.total_disagreeability() as f64;

        1.0 - (disagree - agree).abs() / (disagree + agree)
    }

    fn total_jokes(&self) -> usize {
        self.analysis
            .iter()
            .map(|submission| {
                traverse(submission, &|analysis| analysis.analysis.attitude == Attitude::Mocking || analysis.analysis.subject == Subject::Joke)
                    .into_iter()    
                    .filter(|a| *a)
                    .count()
            })
            .sum()
    }

    fn subject_distribution(&self) -> HashMap<Subject, usize> {
        let distribution = std::sync::Mutex::new(HashMap::new());
        for subject in Subject::VALUES {
            let mut distribution = distribution.lock().unwrap();
            distribution.insert(subject, 0);
        }

        for submission in &self.analysis {
            traverse(submission, &|analysis| {
                let mut distribution = distribution.lock().unwrap();
                distribution.entry(analysis.analysis.subject).and_modify(|count| *count += 1);
            });
        }

        let distribution = distribution.lock().unwrap();
        distribution.clone()
    }

    fn attitude_distribution(&self) -> HashMap<Attitude, usize> {
        let distribution = std::sync::Mutex::new(HashMap::new());
        for subject in Attitude::VALUES {
            let mut distribution = distribution.lock().unwrap();
            distribution.insert(subject, 0);
        }

        for submission in &self.analysis {
            traverse(submission, &|analysis| {
                let mut distribution = distribution.lock().unwrap();
                distribution.entry(analysis.analysis.attitude).and_modify(|count| *count += 1);
            });
        }

        let distribution = distribution.lock().unwrap();
        distribution.clone()
    }

    fn attitude_per_subject_distribution(&self) -> HashMap<(Subject, Attitude), usize> {
        let distribution = std::sync::Mutex::new(HashMap::new());
        for subject in Subject::VALUES {
            for attitude in Attitude::VALUES {
                let mut distribution = distribution.lock().unwrap();
                distribution.insert((subject, attitude), 0);
            }
        }

        for submission in &self.analysis {
            traverse(submission, &|analysis| {
                let mut distribution = distribution.lock().unwrap();
                distribution.entry((analysis.analysis.subject, analysis.analysis.attitude)).and_modify(|count| *count += 1);
            });
        }

        let distribution = distribution.lock().unwrap();
        distribution.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub subreddit_name: String,
    pub subscribers: usize,    
    pub total_comments: usize,
    pub total_positive_comments: usize,
    pub total_negative_comments: usize,
    pub total_agreeability: usize,
    pub total_disagreeability: usize,
    pub divisiveness: f64,
    pub total_jokes: usize,
    pub subject_distribution: HashMap<Subject, usize>,
    pub attitude_distribution: HashMap<Attitude, usize>,
    pub attitude_per_subject_distribution: HashMap<(Subject, Attitude), usize>,
}

impl Stats {
    fn new(subreddit_name: &str) -> Self {
        let data = SubredditData::new(subreddit_name);
        Self {
            subreddit_name: subreddit_name.to_string(),
            subscribers: data.subreddit.subscribers as usize,
            total_comments: data.total_comments(),
            total_positive_comments: data.total_positive_comments(),
            total_negative_comments: data.total_negative_comments(),
            total_agreeability: data.total_agreeability(),
            total_disagreeability: data.total_disagreeability(),
            divisiveness: data.total_divisiveness(),
            total_jokes: data.total_jokes(),
            subject_distribution: data.subject_distribution(),
            attitude_distribution: data.attitude_distribution(),
            attitude_per_subject_distribution: data.attitude_per_subject_distribution(),
        }
    }
}

fn plot_positivity_vs_divisiveness(stats: &[Stats]) -> Result<(), Box<dyn std::error::Error>> {
    let root_area = SVGBackend::new("graphs/positivity-vs-divisiveness.svg", (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE)?;

    let mut cc = ChartBuilder::on(&root_area)
        .margin(35)
        // .margin_top(50)
        // .set_left_and_bottom_label_area_size(35)
        .set_all_label_area_size(50)
        .caption("Positivity vs. Divisiveness", ("sans-serif", 40))
        .build_cartesian_2d(0.0..0.4_f64, 0.25..1.0_f64)?;
    // cc.configure_axes()
    //     .max_light_lines(5).axis_panel_style(GREEN.mix(0.1)).bold_grid_style(BLUE.mix(0.3))
    //     .light_grid_style(BLUE.mix(0.2)).label_style(("Calibri", 10))
    //     .x_formatter(&|x| format!("x={x}")).draw().unwrap();
    cc.configure_mesh()
        .x_desc("Positivity")
        .y_desc("Divisiveness")
        .x_labels(10)
        .y_labels(10)
        .x_label_formatter(&|v| format!("{:.1}", v))
        .y_label_formatter(&|v| format!("{:.1}", v))
        .draw()?;
    
        
    let max_subs = stats.iter().map(|s| s.subscribers).max().unwrap() as f64;
    let max = |a, b| if a > b { a } else { b }; 
    let min = |a, b| if a < b { a } else { b }; 

    let stat_to_elem = |stats: &Stats| {
        let x = stats.total_positive_comments as f64 / stats.total_comments as f64;
        let y = stats.divisiveness;

        let color = RGBColor(
            min((stats.total_disagreeability as f64 / stats.total_comments as f64) * 3.0 * 255.0, 254.0) as u8,
            min((stats.total_jokes as f64 / stats.total_comments as f64) * 3.0 * 255.0, 254.0) as u8,
            min((stats.total_agreeability as f64 / stats.total_comments as f64) * 2.5 * 255.0, 254.0) as u8,
        );

        const FONT_SIZE: i32 = 8;

        return EmptyElement::at((x, y))
            + Circle::new((0, 0), 100.0 * max(stats.subscribers as f64 / max_subs, 0.02), color.mix(0.5).filled())
            + Text::new(format!("r/{}", stats.subreddit_name), (-FONT_SIZE / 2 * stats.subreddit_name.len() as i32 / 2, 2), TextStyle::from(("sans-serif", FONT_SIZE).into_font()).color(&BLACK))
    };

    cc.draw_series(stats.iter().map(stat_to_elem))?
        .label("Subscriber count")
        .legend(|(x, y)| Circle::new((x, y), 5, RGBColor(180, 0, 180).mix(0.5).filled()));

    cc.configure_series_labels().position(SeriesLabelPosition::UpperRight).margin(20)
        .legend_area_size(5).border_style(BLUE).background_style(BLUE.mix(0.1)).label_font(("Calibri", 20)).draw()?;

    root_area.present()?;
    Ok(())
}


fn plot_humor(stats: &[Stats]) -> Result<(), Box<dyn std::error::Error>> {
    let mut stats = stats.to_vec();
    stats.sort_by(|a, b| match (b.total_jokes as f64 / b.total_comments as f64).partial_cmp(&(a.total_jokes as f64 / a.total_comments as f64)) {
        Some(c) => c,
        None => std::cmp::Ordering::Equal,
    });

    let root_area = SVGBackend::new("graphs/humor.svg", (3840, 1080)).into_drawing_area();
    root_area.fill(&WHITE)?;

    let mut cc = ChartBuilder::on(&root_area)
        .margin(35)
        .set_left_and_bottom_label_area_size(130)
        .caption("Humor Content Per Subreddit", ("sans-serif", 100))
        .build_cartesian_2d((0..stats.len() as i32).into_segmented(), 0..25)?;

    let names = stats.iter().map(|s| s.subreddit_name.clone()).collect::<Vec<_>>();
    cc.configure_mesh()
        .x_desc("Subreddit")
        .y_desc("Percentage of Joke Submissions")
        .x_label_style(("sans-serif", 10))
        .y_label_style(("sans-serif", 30))
        .axis_desc_style(("sans-serif", 50))
        .x_labels(stats.len() * 2)
        .x_label_formatter(&|x| {
            let i = match x {
                SegmentValue::Exact(n) | SegmentValue::CenterOf(n) => *n as usize,
                _ => panic!("Unexpected value"),
            };
            if i < names.len() {
                names[i].clone()
            } else {
                "".to_string()
            }
        })
        .y_label_formatter(&|v| format!("{:.1}%", v))
        .draw()?;
    
        
    let max_subs = stats.iter().map(|s| s.subscribers).max().unwrap() as f64;
    let max = |a, b| if a > b { a } else { b };
    let min = |a, b| if a < b { a } else { b }; 

    cc.draw_series(
        Histogram::vertical(&cc)
        .style_func(|x, _bar_height| {
            let i = match x {
                SegmentValue::Exact(n) | SegmentValue::CenterOf(n) => *n as usize,
                _ => panic!("Unexpected value"),
            };
            let stats = &stats[i];
            let color = RGBColor(
                min((stats.total_disagreeability as f64 / stats.total_comments as f64) * 3.0 * 255.0, 254.0) as u8,
                0,
                min((stats.total_agreeability as f64 / stats.total_comments as f64) * 2.5 * 255.0, 254.0) as u8,
            );
            
            color.mix(max(stats.subscribers as f64 / max_subs * 1.2, 0.1)).filled()
        })
        .margin(10)
        .data(stats.iter().enumerate().map(|(i, x)| (i as i32, (x.total_jokes as f64 / x.total_comments as f64 * 100.0) as i32)))
    )?;

    root_area.present()?;
    Ok(())
}


fn rgb(r: f64, g: f64, b: f64) -> RGBColor {
    RGBColor((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> RGBColor {
    let h = h % 360.0;
    if s <= 0.01 { return rgb(v, v, v) };
    let mut i = (h*6.0) as i32; //# XXX assume int() truncates!
    let f = (h*6.0) - i as f64;
    let p = v*(1.-s);
    let q = v*(1.-s*f);
    let t = v*(1.-s*(1.-f));
    i %= 6;
    if i == 0 { rgb(v, t, p) }
    else if i == 1 { rgb(q, v, p) }
    else if i == 2 { rgb(p, v, t) }
    else if i == 3 { rgb(p, q, v) }
    else if i == 4 { rgb(t, p, v) }
    else { rgb(v, p, q)}
}

fn plot_overall_submission_breakdown(stats: &[Stats]) -> Result<(), Box<dyn std::error::Error>> {
    let root_area = SVGBackend::new("graphs/overall-submission-breakdown.svg", (2148, 1124)).into_drawing_area();
    root_area.fill(&WHITE)?;
    let (subject_area, attitude_area) = root_area.split_horizontally(2048 / 2);
    let margin = 35;
    let subject_area = subject_area.margin(margin, margin, margin, margin);
    let attitude_area = attitude_area.margin(margin, margin, margin, margin);
    subject_area.titled("Breakdown of Post Subjects", ("sans-serif", 50))?;
    attitude_area.titled("Breakdown of Post Attitudes", ("sans-serif", 50))?;

    let dims = subject_area.dim_in_pixel();
    let center = (dims.0 as i32 / 2, dims.1 as i32 / 2);
    let radius = 375.0;
    let mut sizes_and_labels = Subject::VALUES.iter().map(
        |s| (stats.iter().map(|x| x.subject_distribution.get(s).unwrap_or(&0)).sum::<usize>() as f64, s.to_string())
    ).filter(|(_, label)| label != "Other").collect::<Vec<_>>();

    sizes_and_labels.sort_by(|(size1, _), (size2, _)| match size2.partial_cmp(&size1) {
        Some(c) => c,
        None => std::cmp::Ordering::Equal,
    });

    let (sizes, labels) = sizes_and_labels.iter().cloned().unzip::<_, _, Vec<_>, Vec<_>>();
    let colors = Subject::VALUES.iter().enumerate().map(|(i, _)| hsv_to_rgb(i as f64 / Subject::VALUES.len() as f64, 1.0, 1.0)).collect::<Vec<_>>();
    let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
    pie.start_angle(10.0);
    pie.label_offset(radius * 0.075);
    pie.label_style((("sans-serif", 18).into_font()).color(&BLACK));
    pie.percentages((("sans-serif", radius * 0.04).into_font()).color(&BLACK));
    subject_area.draw(&pie)?;

    let dims = attitude_area.dim_in_pixel();
    let center = (dims.0 as i32 * 3 / 2, dims.1 as i32 / 2);
    let radius = 375.0;
    let mut sizes_and_labels = Attitude::VALUES.iter().map(
        |s| (stats.iter().map(|x| x.attitude_distribution.get(s).unwrap_or(&0)).sum::<usize>() as f64, s.to_string())
    ).filter(|(_, label)| label != "Neutral").collect::<Vec<_>>();

    sizes_and_labels.sort_by(|(size1, _), (size2, _)| match size2.partial_cmp(&size1) {
        Some(c) => c,
        None => std::cmp::Ordering::Equal,
    });

    let (sizes, labels) = sizes_and_labels.iter().cloned().unzip::<_, _, Vec<_>, Vec<_>>();
    let colors = Attitude::VALUES.iter().enumerate().map(|(i, _)| hsv_to_rgb(i as f64 / Attitude::VALUES.len() as f64 / 1.33 + 0.4, 1.0, 1.0)).collect::<Vec<_>>();
    let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
    pie.start_angle(10.0);
    pie.label_offset(radius * 0.075);
    pie.label_style((("sans-serif", 18).into_font()).color(&BLACK));
    pie.percentages((("sans-serif", radius * 0.04).into_font()).color(&BLACK));
    attitude_area.draw(&pie)?;

    root_area.present()?;
    Ok(())
}

fn plot_breakdown_by_subject(stats: &[Stats]) -> Result<(), Box<dyn std::error::Error>> {
    let root_area = SVGBackend::new("graphs/subject-breakdown.svg", (3800, 2900)).into_drawing_area();
    root_area.fill(&WHITE)?;
    let root_area = root_area.margin(50, 0, 0, 0);
    root_area.titled("Attitudes of Submissions Towards Subjects", ("sans-serif", 120))?;
    let root_area = root_area.margin(200, 0, 100, 100);
    let areas: HashMap<Subject, DrawingArea<SVGBackend, Shift>> = HashMap::from_iter(Subject::VALUES.into_iter().filter(|s| *s != Subject::Other).zip(root_area.split_evenly((3, Subject::VALUES.len() / 3))));
    let mut distribution = HashMap::<(Subject, Attitude), usize>::new();

    // Get the distribution of attitudes for this subject
    for stat in stats {
        for subject in Subject::VALUES {
            for attitude in Attitude::VALUES {
                *distribution.entry((subject, attitude)).or_insert(0) += stat.attitude_per_subject_distribution.get(&(subject, attitude)).unwrap_or(&0);
            }
        }
    }

    // let colors = Attitude::VALUES.iter().enumerate().map(|(i, _)| hsv_to_rgb(i as f64 / Subject::VALUES.len() as f64, 1.0, 1.0)).collect::<Vec<_>>();
    let mut colors = HashMap::new();
    // for (i, attitude) in Attitude::VALUES.iter().enumerate() {
    //     colors.insert(attitude, hsv_to_rgb(i as f64 / Attitude::VALUES.len() as f64, 1.0, 1.0));
    // }

    // Inquisitive,
    // Praise,
    // Condemnation,
    // Agreement,
    // Complaint,
    // Mocking,
    // Disagreement,
    // Annoyed,
    let rgb = RGBColor;
    colors.insert(&Attitude::Inquisitive, rgb(255, 241, 118));
    colors.insert(&Attitude::Praise, rgb(27, 118, 255));
    colors.insert(&Attitude::Condemnation, rgb(255, 29, 35));
    colors.insert(&Attitude::Agreement, rgb(14, 234, 255));
    colors.insert(&Attitude::Complaint, rgb(255, 109, 31));
    colors.insert(&Attitude::Mocking, rgb(219, 165, 7));
    colors.insert(&Attitude::Disagreement, rgb(210, 54, 0));
    colors.insert(&Attitude::Annoyed, rgb(144, 11, 10));
    colors.insert(&Attitude::Neutral, rgb(255, 255, 255));

    for subject in Subject::VALUES {
        if subject == Subject::Other {
            continue;
        }
        let area = areas.get(&subject).unwrap().margin(35, 35, 35, 35);
        area.titled(&format!("Attitudes Towards {}", subject.to_string()), ("sans-serif", 60))?;

        let dims = area.dim_in_pixel();
        let plotters::coord::Shift(pos) = area.as_coord_spec();

        let center = (pos.0 + dims.0 as i32 / 2, pos.1 + dims.1 as i32 / 2);
        let radius = 250.0;
        let mut sizes_labels_and_colors = Attitude::VALUES.iter().map(
            |attitude| (*distribution.get(&(subject, *attitude)).unwrap_or(&0), attitude.to_string(), *colors.get(attitude).unwrap())
        ).filter(|(size, label, _)| *size >= 1 && label != "Neutral").collect::<Vec<_>>();
    
        sizes_labels_and_colors.sort_by(|(size1, _, _), (size2, _, _)| match size2.partial_cmp(&size1) {
            Some(c) => c,
            None => std::cmp::Ordering::Equal,
        });
    
        let (sizes, labels_and_colors) = sizes_labels_and_colors.iter().map(|(size, label, color)| (*size as f64, (label.clone(), *color))).unzip::<_, _, Vec<_>, Vec<_>>();
        let (labels, colors) = labels_and_colors.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
        let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
        pie.start_angle(180.0);
        pie.label_offset(radius * 0.075);
        pie.label_style((("sans-serif", 35).into_font()).color(&BLACK));
        pie.percentages((("sans-serif", radius * 0.11).into_font()).color(&BLACK));

        area.draw(&pie)?;
    }
    root_area.present()?;
 
    Ok(())
}

fn plot_reddit_surface(stats: &[Stats]) -> Result<(), Box<dyn std::error::Error>> {
    // let root_area = SVGBackend::new("graphs/surface.svg", (1024, 800)).into_drawing_area();

    let x_dim = Subject::VALUES.len() as i32 - 1;
    let y_dim = 1000;
    let z_dim = Attitude::VALUES.len() as i32 - 1;

    let root_area = BitMapBackend::gif("graphs/subject-attitude-occurences.gif", (1024, 800), 100)?.into_drawing_area();
    root_area.fill(&WHITE)?;
    let root_area = root_area.margin(30, 0, 0, 0);
    root_area.titled("Total Submissions by Subject and Attitude", ("sans-serif", 35))?;
    let root_area = root_area.margin(70, 0, 0, 0);

    let mut lookup_table = HashMap::new();
    for subject in Subject::VALUES {
        for attitude in Attitude::VALUES {
            lookup_table.insert((subject, attitude), stats.iter().map(|stat| stat.attitude_per_subject_distribution.get(&(subject, attitude)).unwrap_or(&0)).sum::<usize>() as i32);
        }
    }

    for pitch in 0..110 {
        root_area.fill(&WHITE)?;
    
        let mut chart_context = ChartBuilder::on(&root_area)
            .margin(10)
            .build_cartesian_3d(1..x_dim, 0..y_dim, 1..z_dim)
            .unwrap();

        chart_context.with_projection(|mut p| {
            p.pitch = 1.1 - (1.1 - pitch as f64 / 50.0).abs();
            p.scale = 0.7;
            p.into_matrix() // build the projection matrix
        });

        let axis_title_style = ("sans-serif", 20, &BLACK).into_text_style(&root_area);
        chart_context.draw_series([
            ("Subject", (x_dim + 2, 0, 1)),
            ("Occurences", (1, y_dim + 150, 1)),
            ("Attitude", (1, 0, z_dim + 2)),
        ]
        .map(|(label, position)| Text::new(label, position, &axis_title_style))).unwrap();
        chart_context.draw_series(SurfaceSeries::xoz(
            (1..=x_dim).map(|v| v),
            (1..=z_dim).map(|v| v),
            |x:i32,z:i32| {
                let subject = Subject::VALUES[x as usize];
                let attitude = Attitude::VALUES[z as usize];
                *lookup_table.get(&(subject, attitude)).unwrap_or(&0)
            }
        ).style_func(
            &|y| {
                HSLColor(*y as f64 / y_dim as f64, 0.6666, 0.5).mix(0.7).filled()
            }
        )).unwrap();

        chart_context.configure_axes().tick_size(8)
            .x_labels(x_dim as usize)
            .z_labels(z_dim as usize)
            .max_light_lines(3)
            .axis_panel_style(PURPLE.mix(0.1))
            .bold_grid_style(BLACK.mix(0.3))
            .light_grid_style(BLUE.mix(0.2))
            .x_formatter(&|x| format!("{:?}", Subject::VALUES[*x as usize]))
            .y_formatter(&|y| format!("{y}"))
            .z_formatter(&|z| format!("{:?}", Attitude::VALUES[*z as usize]))
            .draw()?;

        root_area.present()?;
    }

    root_area.present()?;

    Ok(())
}

fn plot_subreddit_breakdown(stats: &[Stats], subreddits: &[&[&str]]) -> Result<(), Box<dyn std::error::Error>> {
    let root_area = SVGBackend::new("graphs/subreddit-breakdown.svg", (10000, 5300)).into_drawing_area();
    root_area.fill(&WHITE)?;
    let root_area = root_area.margin(200, 0, 0, 0);
    root_area.titled("Attitudes and Subjects by Subreddit", ("sans-serif", 180))?;
    let root_area = root_area.margin(400, 0, 100, 100);

    let mut subreddit_stats = HashMap::new();
    for stat in stats {
        subreddit_stats.insert(stat.subreddit_name.to_string(), stat);
    }

    let rows = subreddits.len();
    let cols = subreddits.iter().map(|subreddits| subreddits.len()).max().unwrap();
    let raw_areas = root_area.split_evenly((rows, 2 * cols));
    
    let mut areas = HashMap::new();
    for row in 0..rows {
        for col in 0..cols {
            let subject_area = raw_areas[row * (cols * 2) + col * 2].clone();
            let attitude_area = raw_areas[row * (cols * 2) + col * 2 + 1].clone();
            areas.insert((row, col * 2), subject_area);
            areas.insert((row, col * 2 + 1), attitude_area);
        }
    }
    let mut attitude_colors = HashMap::new();
    let rgb = RGBColor;
    attitude_colors.insert(&Attitude::Inquisitive, rgb(255, 241, 118));
    attitude_colors.insert(&Attitude::Praise, rgb(27, 118, 255));
    attitude_colors.insert(&Attitude::Condemnation, rgb(255, 29, 35));
    attitude_colors.insert(&Attitude::Agreement, rgb(14, 234, 255));
    attitude_colors.insert(&Attitude::Complaint, rgb(255, 109, 31));
    attitude_colors.insert(&Attitude::Mocking, rgb(219, 165, 7));
    attitude_colors.insert(&Attitude::Disagreement, rgb(210, 54, 0));
    attitude_colors.insert(&Attitude::Annoyed, rgb(144, 11, 10));
    attitude_colors.insert(&Attitude::Neutral, rgb(255, 255, 255));

    let mut subject_colors = HashMap::new();
    for (i, subject) in Subject::VALUES.into_iter().enumerate() {
        subject_colors.insert(subject, hsv_to_rgb(i as f64 / Subject::VALUES.len() as f64, 1.0, 1.0));
    }

    for (row, row_of_subreddits) in subreddits.iter().enumerate() {
        for (col, subreddit) in row_of_subreddits.iter().enumerate() {
            let subject_area = areas.get(&(row, col * 2)).unwrap();
            let attitude_area = areas.get(&(row, col * 2 + 1)).unwrap();
            subject_area.titled(&format!("r/{subreddit} Subjects"), ("sans-serif", 100))?;
            attitude_area.titled(&format!("r/{subreddit} Attitudes"), ("sans-serif", 100))?;

            let dims = subject_area.dim_in_pixel();
            let plotters::coord::Shift(pos) = subject_area.as_coord_spec();

            let radius = 500.0;
            let center = (pos.0 + dims.0 as i32 / 2, pos.1 + dims.1 as i32 / 2);

            let mut sizes_labels_and_colors = Subject::VALUES.iter().map(
                |s| (*subreddit_stats.get(&subreddit.to_string()).unwrap().subject_distribution.get(s).unwrap_or(&0) as f64, s.to_string(), subject_colors.get(s).unwrap())
            ).filter(|(size, label, _)| *size > 0.0 && label != "Other").collect::<Vec<_>>();
            sizes_labels_and_colors.sort_by(|(size1, _, _), (size2, _, _)| match size2.partial_cmp(&size1) {
                Some(c) => c,
                None => std::cmp::Ordering::Equal,
            });
            let (sizes, labels_and_colors) = sizes_labels_and_colors.iter().cloned().map(|(size, label, color)| (size, (label, *color))).unzip::<_, _, Vec<_>, Vec<_>>();
            let (labels, colors) = labels_and_colors.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();
            let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
            pie.start_angle(180.0);
            pie.label_offset(radius * 0.075);
            pie.label_style((("sans-serif", 70).into_font()).color(&BLACK));
            pie.percentages((("sans-serif", radius * 0.15).into_font()).color(&BLACK));
            subject_area.draw(&pie)?;

            let dims = attitude_area.dim_in_pixel();
            let plotters::coord::Shift(pos) = attitude_area.as_coord_spec();
            let center = (pos.0 + dims.0 as i32 / 2, pos.1 + dims.1 as i32 / 2);

            let mut sizes_labels_and_colors = Attitude::VALUES.iter().map(
                |a| (*subreddit_stats.get(&subreddit.to_string()).unwrap().attitude_distribution.get(a).unwrap_or(&0) as f64, a.to_string(), attitude_colors.get(a).unwrap())
            ).filter(|(size, label, _)| *size > 0.0 && label != "Neutral").collect::<Vec<_>>();
            sizes_labels_and_colors.sort_by(|(size1, _, _), (size2, _, _)| match size2.partial_cmp(&size1) {
                Some(c) => c,
                None => std::cmp::Ordering::Equal,
            });
            let (sizes, labels_and_colors) = sizes_labels_and_colors.iter().cloned().map(|(size, label, color)| (size, (label, *color))).unzip::<_, _, Vec<_>, Vec<_>>();
            let (labels, colors) = labels_and_colors.into_iter().unzip::<_, _, Vec<_>, Vec<_>>();

            let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
            pie.start_angle(180.0);
            pie.label_offset(radius * 0.075);
            pie.label_style((("sans-serif", 70).into_font()).color(&BLACK));
            pie.percentages((("sans-serif", radius * 0.15).into_font()).color(&BLACK));
            attitude_area.draw(&pie)?;
        }
    }
    root_area.present()?;
 
    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = std::fs::read_dir("./data").unwrap();
    let mut stats = vec![];
    for path in paths {
        let path = path.unwrap().path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        if std::path::Path::new(&format!("analysis/{}_subreddit_analysis.json", name)).exists() {
            let stat = Stats::new(name);
            stats.push(stat);
        }
    }

    // stats.sort_by(|a, b| (b.total_jokes as f64 / b.total_comments as f64).partial_cmp(&(a.total_jokes as f64 / a.total_comments as f64)).unwrap());
    plot_subreddit_breakdown(&stats, &[
        &["aww", "rage", "jokes"],
        &["pics", "gaming", "inceltear"],
        &["memeeconomy", "therewasanattempt", "outoftheloop"],
    ])?;
    plot_positivity_vs_divisiveness(&stats)?;
    plot_humor(&stats)?;
    plot_overall_submission_breakdown(&stats)?;
    plot_breakdown_by_subject(&stats)?;
    plot_reddit_surface(&stats)?;
    Ok(())
}