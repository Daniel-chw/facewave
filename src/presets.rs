// quality presets for the cli

/// (levels_s, levels_d, step_s, step_d) for quality 1..=10.
/// Generated from the sweep's Pareto front and will be regenerated.
pub const PRESETS: [(u8, u8, f32, f32); 10] = [
    (4, 1, 2.0, 1.0),
    (4, 1, 1.0, 1.0),
    (5, 4, 0.5, 0.5),
    (4, 4, 0.25, 0.5),
    (4, 4, 0.125, 0.5),
    (4, 4, 0.125, 0.125),
    (4, 4, 0.0625, 0.125),
    (4, 4, 0.0625, 0.0625),
    (4, 4, 0.03125, 0.03125),
    (4, 3, 0.015625, 0.015625),
];
