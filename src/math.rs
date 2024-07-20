use std::time::Instant;
use std::sync::Mutex;
use rayon::prelude::*;

pub fn static_align(target_trace: usize, traces: &[Vec<(f64, f64)>], sample_selection: std::ops::Range<usize>, max_distance: usize, correlation_threshold: f64) -> Result<(Vec::<(usize, i64, f64)>), String> {

    let target= &traces[target_trace][sample_selection.clone()];
    let half_selection = (sample_selection.len() as f64 / 2.0).ceil() as i64;

    let start = Instant::now();

    let min = (sample_selection.start as i64 - max_distance as i64 + half_selection)
        .clamp(0 + half_selection, sample_selection.end as i64 - half_selection) as usize;
    let max = (sample_selection.end as i64 + max_distance as i64 - half_selection)
        .clamp(0 + half_selection, sample_selection.end as i64 - half_selection) as usize;
    // Collect traces that match with (trace number, shift amount, correlation)
    let mut all_correlations = Mutex::new(Vec::<Vec<f64>>::new());
    let mut matching_traces = Mutex::new(Vec::<(usize, i64, f64)>::new());

    (min..=max).into_par_iter().for_each(|index|{
        let correlations = calculate_correlation(
            target_trace,
            &target,
            &traces,
            (index-half_selection as usize)..(index+half_selection as usize),
        );

        let mut all_correlations = all_correlations.lock().unwrap();
        all_correlations.push(correlations);
    });

    all_correlations.lock().unwrap().iter().for_each(|correlations|{
        correlations.iter().enumerate().for_each(|(i, r)| {
            let mut matching_traces = matching_traces.lock().unwrap();
            if r >= &correlation_threshold {
                let trace = (i, (i as i64 - target_trace as i64), *r);
                matching_traces.push(trace);
            }
        });
    });

    let time = Instant::now() - start;

    log::info!("Static Align Elapsed Time: {:?}", time);
    let mut matching_traces = matching_traces.lock().unwrap();

    let compare = |a: &(usize, i64, f64), b: &(usize, i64, f64)| a.0.partial_cmp(&b.0).unwrap();
    matching_traces.sort_by(compare);
    Ok(matching_traces.clone())
}

/// Calculates the correlation between selected samples from the target_trace and every other trace and returns the values
pub fn calculate_correlation(target_index: usize, target_samples: &[(f64, f64)], traces: &[Vec<(f64, f64)>], selection: std::ops::Range<usize>) -> Vec<f64> {

    // Define helper functions for calculating variance, average, and standard deviation
    let length = traces.len() as f64;
    let split_y = |trace: &[(f64, f64)]| trace.par_iter().map(|var| var.1).collect::<Vec<f64>>();
    let avg = move |x: &[f64]| x.par_iter().sum::<f64>() / length;
    let variance = move |x: &[f64], avg: f64| x.par_iter().map(|val| val - avg).collect::<Vec<f64>>();
    let standard_deviation = |x: &[f64], avg: f64| x.par_iter().map(|var| (var - avg).powf(2.0)).sum::<f64>().sqrt();

    // Calculate the target trace standard deviation and variance
    let target_y = split_y(&target_samples);
    let target_mean: f64 = avg(&target_y);
    let target_variance = variance(&target_y, target_mean);
    let target_stan_deviation = standard_deviation(&target_y, target_mean);

    // Hold our correlation values
    let mut correlations = Mutex::new(vec![0.0; length as usize]);

    // Iterate through other traces
    traces.par_iter().enumerate().for_each(|(index, trace)| {
            if index != target_index {
                let trace_y = split_y(&trace[selection.clone()]);
                let trace_mean: f64 = avg(&trace_y);
                let trace_variance = variance(&trace_y, trace_mean);
                let trace_stan_deviation = standard_deviation(&trace_y, trace_mean);

                let mut r = target_variance.par_iter().enumerate().map(|(i, var)| var * trace_variance[i]).sum::<f64>();
                r /= target_stan_deviation * trace_stan_deviation;

                let mut correlations = correlations.lock().unwrap();
                correlations[index] = r;
            }
    });

    let correlations = correlations.lock().unwrap();

    correlations.to_vec()
}