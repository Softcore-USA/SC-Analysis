mod wave;
use plotters::backend::DrawingBackend;
use plotters::prelude::*;
use plotters::prelude::full_palette::{BLUE_300, BLUE_50, BLUEGREY_900};
use wave::*;
fn main() {
    let ws = SinWaveDefinition {
        sample_delta: 0.01,
        phase_shift: 0.0,
        vertical_shift: 0.0,
        amplitude: 0.1,
        samples: 10000,
    };

    let mut wave = Wave::new();
    wave.generate_sin_wave(ws.clone());

    let root = BitMapBackend::new("plotters-doc-data/0.png", (1920, 1080)).into_drawing_area();
    root.fill(&BLUEGREY_900).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Complex Wave Data", ("sans-serif", 50).into_font().color(&BLUE_300))
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(-0.1f32..(ws.samples as f32 * ws.sample_delta), -(ws.amplitude + ws.vertical_shift)..(ws.amplitude + ws.vertical_shift)).unwrap();

    chart.configure_mesh().axis_style(&BLUE_50).draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            wave.data_points.iter().map(|p| (p.0, p.1)),
            &BLUE_300,
        )).unwrap()
        .label(format!("sinwave"))
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE_300));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&WHITE)
        .draw().unwrap();

    root.present().unwrap();
}
