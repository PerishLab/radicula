const KEEP: [f64; 5] = [1.0, 0.99, 0.9, 0.75, 0.5];

fn welch(first: &[f64], second: &[f64]) -> f64 {
    let (mean_a, var_a) = moments(first);
    let (mean_b, var_b) = moments(second);
    let spread = var_a / first.len() as f64 + var_b / second.len() as f64;
    (mean_a - mean_b) / spread.sqrt()
}

fn moments(values: &[f64]) -> (f64, f64) {
    let count = values.len() as f64;
    let mean = values.iter().sum::<f64>() / count;
    let var = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    (mean, var)
}

pub fn score(samples: &[(bool, f64)]) -> f64 {
    let mut sorted: Vec<f64> = samples.iter().map(|sample| sample.1).collect();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let mut worst = 0.0f64;
    for keep in KEEP {
        let edge = sorted[((sorted.len() - 1) as f64 * keep) as usize];
        let pick = |class: bool| -> Vec<f64> {
            samples
                .iter()
                .filter(|sample| sample.0 == class && sample.1 <= edge)
                .map(|sample| sample.1)
                .collect()
        };
        let value = welch(&pick(false), &pick(true)).abs();
        worst = worst.max(value);
    }
    worst
}

pub fn sides(samples: &[(bool, f64)]) -> [f64; 4] {
    let take = |class: bool| -> Vec<f64> {
        samples
            .iter()
            .filter(|sample| sample.0 == class)
            .map(|sample| sample.1)
            .collect()
    };
    let (one, first) = moments(&take(false));
    let (two, second) = moments(&take(true));
    [one, two, first.sqrt(), second.sqrt()]
}
