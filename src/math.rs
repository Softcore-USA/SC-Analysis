use rayon::prelude::*;
use std::ops::Range;
use std::sync::Mutex;
use std::time::Instant;

// pub(crate) fn compute_static_alignment(
//     target_trace: usize,
//     traces: &[Vec<(f64, f64)>],
//     sample_selection: Range<usize>,
//     max_distance: usize,
//     correlation_threshold: f64,
// ) -> Vec<Vec<(f64, f64)>> {
//     // Convert target trace to ArrayFire array
//     let target: Vec<f64> = traces[target_trace]
//         .iter()
//         .skip(sample_selection.start)
//         .take(sample_selection.end - sample_selection.start)
//         .map(|&(_, y)| y)
//         .collect();
//
//     let target_array = Array::new(&target, Dim4::new(&[target.len() as u64, 1, 1, 1]));
//
//     // Clamp the range to valid bounds
//     let search_start = if sample_selection.start > max_distance {
//         sample_selection.start - max_distance
//     } else {
//         0
//     };
//
//     let search_end = (sample_selection.end + max_distance).min(traces[target_trace].len());
//
//     let search_range = search_start..search_end;
//
//     // Convert all traces to a batched ArrayFire array
//     let batched_traces: Vec<f64> = traces
//         .iter()
//         .filter(|trace| trace != &&traces[target_trace])
//         .flat_map(|trace| {
//             trace[search_range.clone()]
//                 .iter()
//                 .map(|&(_, y)| y)
//
//         })
//         .collect();
//
//     let num_traces = traces.len() - 1;
//
//
//     let batched_array = Array::new(&batched_traces, Dim4::new(&[search_range.len() as u64, num_traces as u64, 1, 1]));
//
//     let batched_array = flip(&batched_array, 0);
//
//     // Cross-correlation using FFT convolution
//     let corr = fft_convolve1(&target_array, &batched_array, ConvMode::EXPAND);
//
//     let corr = flip(&corr, 0);
//
//     let sum_of_squares_target = sum(&(&target_array * &target_array), 0);
//     let norm_0th_dim_target = sqrt(&sum_of_squares_target);
//
//     let sum_of_squares_batched = sum(&(&batched_array * &batched_array), 0);
//     let norm_0th_dim_batched = sqrt(&sum_of_squares_batched);
//
//     let norm_corr = &corr / (&norm_0th_dim_target * &norm_0th_dim_batched);
//
//
//     let (max_value, max_index) = imax(&norm_corr, 0);
//     let mut max_corr = vec![0.0; max_value.elements()];
//     max_value.host(&mut max_corr);
//     let mut max_corr_index: Vec<u32> = vec![0; max_index.elements()];
//     max_index.host(&mut max_corr_index);
//
//     let center_index = (corr.dims()[0] / 2) as u32;
//
//     let num_cols = traces.len();
//
//     let nested_pairs: Vec<_> = (0..num_cols).into_par_iter().map(|i| {
//         if i == target_trace {
//             traces[i].clone()
//         } else {
//             let shift_amount = if i > 0 && max_corr[i - 1] > correlation_threshold{
//                 ((max_corr_index[i - 1] as isize) - center_index as isize) as i32
//             } else {
//                 0 // Assuming you need some default or no shift for the first index or specific cases
//             };
//             shift_x_values(traces[i].clone(), shift_amount)
//         }
//     }).collect();
//
//     nested_pairs
// }
#[allow(dead_code)]
fn shift_x_values(data: Vec<(f64, f64)>, shift: i32) -> Vec<(f64, f64)> {
    let n = data.len();
    let mut new_xs = vec![0.0; n]; // Temporarily store new x values

    for (index, (x, _y)) in data.iter().enumerate() {
        let new_index = if shift > 0 {
            // Right shift
            (index + shift as usize) % n
        } else {
            // Left shift, with positive modulo handling
            (n + (index as i32 + shift) as usize % n) % n
        };
        new_xs[new_index] = *x;
    }

    // Pair new x values with original y values
    data.iter()
        .zip(new_xs)
        .map(|(&(_, y), x)| (x, y))
        .collect()
}

#[allow(dead_code)]
pub fn static_align(
    target_trace: usize,
    traces: &[Vec<(f64, f64)>],
    sample_selection: Range<usize>,
    max_distance: usize,
    correlation_threshold: f64,
) -> Result<Vec<(usize, i64, f64)>, String> {
    let target = &traces[target_trace][sample_selection.clone()];
    let half_selection = (sample_selection.len() as f64 / 2.0).ceil() as i64;

    let start = Instant::now();

    let min = (sample_selection.start as i64 - max_distance as i64 + half_selection).clamp(
        half_selection,
        sample_selection.end as i64 - half_selection,
    ) as usize;
    let max = (sample_selection.end as i64 + max_distance as i64 - half_selection).clamp(
        half_selection,
        sample_selection.end as i64 - half_selection,
    ) as usize;
    // Collect traces that match with (trace number, shift amount, correlation)
    let all_correlations = Mutex::new(Vec::<Vec<f64>>::new());
    let matching_traces = Mutex::new(Vec::<(usize, i64, f64)>::new());

    (min..=max).into_par_iter().for_each(|index| {
        let correlations = calculate_correlation(
            target_trace,
            target,
            traces,
            (index - half_selection as usize)..(index + half_selection as usize),
        );

        let mut all_correlations = all_correlations.lock().unwrap();
        all_correlations.push(correlations);
    });

    all_correlations
        .lock()
        .unwrap()
        .iter()
        .for_each(|correlations| {
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

#[allow(dead_code)]
/// Calculates the correlation between selected samples from the target_trace and every other trace and returns the values
pub fn calculate_correlation(
    target_index: usize,
    target_samples: &[(f64, f64)],
    traces: &[Vec<(f64, f64)>],
    selection: std::ops::Range<usize>,
) -> Vec<f64> {
    // Define helper functions for calculating variance, average, and standard deviation
    let length = traces.len() as f64;
    let split_y = |trace: &[(f64, f64)]| trace.par_iter().map(|var| var.1).collect::<Vec<f64>>();
    let avg = move |x: &[f64]| x.par_iter().sum::<f64>() / length;
    let variance =
        move |x: &[f64], avg: f64| x.par_iter().map(|val| val - avg).collect::<Vec<f64>>();
    let standard_deviation = |x: &[f64], avg: f64| {
        x.par_iter()
            .map(|var| (var - avg).powf(2.0))
            .sum::<f64>()
            .sqrt()
    };

    // Calculate the target trace standard deviation and variance
    let target_y = split_y(target_samples);
    let target_mean: f64 = avg(&target_y);
    let target_variance = variance(&target_y, target_mean);
    let target_stan_deviation = standard_deviation(&target_y, target_mean);

    // Hold our correlation values
    let correlations = Mutex::new(vec![0.0; length as usize]);

    // Iterate through other traces
    traces.par_iter().enumerate().for_each(|(index, trace)| {
        if index != target_index {
            let trace_y = split_y(&trace[selection.clone()]);
            let trace_mean: f64 = avg(&trace_y);
            let trace_variance = variance(&trace_y, trace_mean);
            let trace_stan_deviation = standard_deviation(&trace_y, trace_mean);

            let mut r = target_variance
                .par_iter()
                .enumerate()
                .map(|(i, var)| var * trace_variance[i])
                .sum::<f64>();
            r /= target_stan_deviation * trace_stan_deviation;

            let mut correlations = correlations.lock().unwrap();
            correlations[index] = r;
        }
    });

    let correlations = correlations.lock().unwrap();

    correlations.to_vec()
}
