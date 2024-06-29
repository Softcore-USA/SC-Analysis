#[derive(Clone)]
pub struct SinWaveDefinition {
    pub sample_delta: f32,
    pub phase_shift: f32,
    pub vertical_shift: f32,
    pub amplitude: f32,
    pub samples: i32,
}

impl Default for SinWaveDefinition {
    fn default() -> Self {
        SinWaveDefinition {
            sample_delta: 0.0,
            phase_shift: 0.0,
            vertical_shift: 0.0,
            amplitude: 0.0,
            samples: 0
        }
    }
}

#[derive(Clone)]
pub struct Wave {
    pub data_points: Vec<(f32, f32)>,
}

impl Wave {
    pub fn new() -> Wave {
        Wave {
            data_points: vec![],
        }
    }

    pub fn generate_sin_wave(&mut self, wave: SinWaveDefinition) {
        for i in 0..wave.samples {
            self.data_points.push(
                (
                    i as f32 * wave.sample_delta ,
                (((i as f32 - wave.phase_shift) * wave.sample_delta).sin() * wave.amplitude) + wave.vertical_shift
                )
            );
        }
    }
}
