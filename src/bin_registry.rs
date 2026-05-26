use std::ffi::CStr;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinParamType {
    F32,
    Color3,
}

#[derive(Clone, Copy, Debug)]
pub struct BinParamSpec {
    pub id: &'static CStr,
    pub label: &'static CStr,
    pub param_type: BinParamType,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub flags: u32,
}

impl BinParamSpec {
    pub fn id_str(&self) -> &'static str {
        self.id.to_str().unwrap_or("")
    }

    pub fn label_str(&self) -> &'static str {
        self.label.to_str().unwrap_or("")
    }
}

macro_rules! cstr {
    ($value:literal) => {
        unsafe { CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

macro_rules! f32_param {
    ($id:literal, $label:literal, $min:expr, $max:expr, $default:expr) => {
        BinParamSpec {
            id: cstr!($id),
            label: cstr!($label),
            param_type: BinParamType::F32,
            min_value: $min,
            max_value: $max,
            default_value: $default,
            flags: 0,
        }
    };
}

macro_rules! color3_param {
    ($id:literal, $label:literal, $min:expr, $max:expr, $default:expr) => {
        BinParamSpec {
            id: cstr!($id),
            label: cstr!($label),
            param_type: BinParamType::Color3,
            min_value: $min,
            max_value: $max,
            default_value: $default,
            flags: 0,
        }
    };
}

pub const ROTO_PARAMS: &[BinParamSpec] = &[
    f32_param!("square_size", "Square Size", 0.05, 0.5, 0.2),
    f32_param!("circle_radius", "Circle Radius", 0.05, 0.2, 0.11),
    f32_param!("edge_thickness", "Edge Thickness", 0.001, 0.01, 0.003),
    f32_param!("animation_speed", "Animation Speed", 1.0, 30.0, 12.0),
    color3_param!("background_color", "Background Color", 0.0, 1.0, 0.5),
    f32_param!("edge_color_intensity", "Edge Brightness", 0.1, 2.0, 1.0),
];

pub const CUNEUS_PARAMS: &[BinParamSpec] = &[
    f32_param!("background_color", "Background", 0.0, 1.0, 0.4),
    color3_param!("hue_color", "Base Color", 0.0, 3.0, 1.0),
    f32_param!("light_intensity", "Light Intensity", 0.0, 3.2, 1.8),
    f32_param!("ao_strength", "AO Strength", 0.0, 10.0, 0.1),
    f32_param!("global_light", "Global Light", 0.1, 2.0, 1.0),
    f32_param!("rim_power", "Rim Power", 0.1, 10.0, 3.0),
    f32_param!("env_light_strength", "Environment Light", 0.0, 1.0, 0.5),
    f32_param!("alpha_threshold", "Alpha Threshold", 0.0, 3.0, 1.0),
    f32_param!("mix_factor_scale", "Mix Factor Scale", 0.0, 1.5, 0.3),
    f32_param!("iridescence_power", "Iridescence", 0.0, 1.0, 0.2),
    f32_param!("falloff_distance", "Light Falloff", 0.5, 5.0, 1.0),
];

pub const SPIRAL_PARAMS: &[BinParamSpec] = &[
    f32_param!("lambda", "Lambda", 1.0, 360.0, 35.0),
    f32_param!("theta", "Theta", -6.2, 6.2, 0.7),
    f32_param!("alpha", "Alpha", 0.0, 1.0, 0.7),
    f32_param!("sigma", "Sigma", 0.0, 1.0, 0.1),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.1),
    f32_param!("blue", "Blue", 0.0, 1.0, 0.1),
    f32_param!("use_texture_colors", "Use Texture Colors", 0.0, 1.0, 0.0),
];

pub const VORONOI_PARAMS: &[BinParamSpec] = &[
    f32_param!("scale", "Cell Scale", 1.0, 100.0, 24.0),
    f32_param!("offset_value", "Pattern Offset", -1.0, 2.0, -1.0),
    f32_param!("cell_index", "Cell Index", 0.0, 3.0, 0.0),
    f32_param!("edge_width", "Edge Width", 0.0, 1.0, 0.1),
    f32_param!("highlight", "Edge Highlight", 0.0, 15.0, 0.15),
];

pub const MATRIX_PARAMS: &[BinParamSpec] = &[
    f32_param!("red_power", "Red Power", 0.5, 3.0, 0.98),
    f32_param!("green_power", "Green Power", 0.5, 3.0, 0.85),
    f32_param!("blue_power", "Blue Power", 0.5, 3.0, 0.90),
    f32_param!("green_boost", "Green Boost", 0.5, 2.0, 1.62),
    f32_param!("contrast", "Contrast", 0.5, 2.0, 1.0),
    f32_param!("gamma", "Gamma", 0.2, 2.0, 1.0),
    f32_param!("glow", "Glow", -1.0, 1.0, 0.05),
];

pub const TREE_PARAMS: &[BinParamSpec] = &[
    f32_param!("pixel_offset", "Pixel Offset Y", -3.14, 3.14, -1.0),
    f32_param!("pixel_offset2", "Pixel Offset X", -3.14, 3.14, 1.0),
    f32_param!("lights", "Lights", 0.0, 12.2, 2.2),
    f32_param!("exp", "Exp", 1.0, 120.0, 4.0),
    f32_param!("frame", "Frame", 0.0, 2.2, 1.0),
    f32_param!("col1", "Iter", 0.0, 300.0, 100.0),
    f32_param!("col2", "Col2", 0.0, 10.0, 1.0),
    f32_param!("decay", "Feedback", 0.0, 1.0, 1.0),
];

pub const NEURON2D_PARAMS: &[BinParamSpec] = &[
    f32_param!("pixel_offset", "Pixel Offset Y", -3.14, 3.14, -1.0),
    f32_param!("pixel_offset2", "Pixel Offset X", -3.14, 3.14, 1.0),
    f32_param!("lights", "Lights", 0.0, 12.2, 2.2),
    f32_param!("exp", "Exp", 1.0, 120.0, 4.0),
    f32_param!("frame", "Frame", 0.0, 5.2, 1.0),
    f32_param!("col1", "Iter", 0.0, 150.0, 100.0),
    f32_param!("col2", "Col2", 0.0, 20.0, 1.0),
    f32_param!("decay", "Feedback", 0.0, 1.0, 1.0),
];

pub const GABOR_PARAMS: &[BinParamSpec] = &[
    f32_param!("frequency", "Frequency", 0.1, 10.0, 3.0),
    f32_param!("orientation", "Orientation", -3.1415927, 3.1415927, 0.0),
    f32_param!("phase", "Phase", -3.1415927, 3.1415927, 0.0),
    f32_param!("speed", "Speed", 0.0, 5.0, 1.0),
    f32_param!("sigma_x", "Sigma X", 0.1, 3.0, 1.0),
    f32_param!("sigma_y", "Sigma Y", 0.1, 3.0, 1.0),
    f32_param!("amplitude", "Amplitude", 0.0, 2.0, 1.0),
    f32_param!("aspect_ratio", "Aspect Ratio", 0.5, 2.0, 1.0),
    f32_param!("z_scale", "Z Depth Scale", 0.0, 1.0, 0.25),
    f32_param!("brightness", "Brightness", 0.00001, 0.0001, 0.00003),
    f32_param!("rotation_x", "Rotation X", -1.0, 1.0, 0.0),
    f32_param!("rotation_y", "Rotation Y", -1.0, 1.0, 0.0),
    f32_param!("dof_amount", "DOF Amount", 0.0, 3.0, 0.0),
    f32_param!("dof_focal_dist", "Focal Distance", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Positive Red", 0.0, 1.0, 0.0),
    f32_param!("color1_g", "Positive Green", 0.0, 1.0, 0.7),
    f32_param!("color1_b", "Positive Blue", 0.0, 1.0, 1.0),
    f32_param!("color2_r", "Negative Red", 0.0, 1.0, 1.0),
    f32_param!("color2_g", "Negative Green", 0.0, 1.0, 0.3),
    f32_param!("color2_b", "Negative Blue", 0.0, 1.0, 0.0),
];

pub const PLASMA_PARAMS: &[BinParamSpec] = &[
    f32_param!("detail", "Detail", 3.0, 45.0, 18.0),
    f32_param!("animation_speed", "Animation Speed", 0.1, 6.0, 1.0),
    f32_param!("pattern", "Pattern", 0.0, 1.0, 0.0),
    f32_param!("structure_smoothness", "Smoothness", 1.0, 3.5, 2.0),
    f32_param!("saturation", "Saturation", 0.1, 1.0, 0.7),
    f32_param!("base_rotation", "Rotation", 3.0, 12.0, 6.0),
    f32_param!("rot_variation", "Rotation Variation", 0.0, 0.1, 0.02),
    f32_param!("brightness_mult", "Brightness", 0.00001, 0.0001, 0.00003),
    f32_param!("rotation_x", "Rotation X", -1.0, 1.0, 0.0),
    f32_param!("rotation_y", "Rotation Y", -1.0, 1.0, 0.0),
    f32_param!("dof_amount", "DOF Amount", 0.0, 3.0, 0.0),
    f32_param!("dof_focal_dist", "Focal Distance", 0.0, 3.0, 1.0),
    f32_param!("color1_r", "Base Red", 0.0, 1.0, 0.5),
    f32_param!("color1_g", "Base Green", 0.0, 1.0, 0.1),
    f32_param!("color1_b", "Base Blue", 0.0, 1.0, 0.8),
    f32_param!("color2_r", "Highlight Red", 0.0, 1.0, 0.0),
    f32_param!("color2_g", "Highlight Green", 0.0, 1.0, 0.7),
    f32_param!("color2_b", "Highlight Blue", 0.0, 1.0, 1.0),
];

pub const LORENZ_PARAMS: &[BinParamSpec] = &[
    f32_param!("sigma", "Sigma", 0.0, 40.0, 10.0),
    f32_param!("rho", "Rho", 0.0, 100.0, 28.0),
    f32_param!("beta", "Beta", 0.0, 10.0, 2.6666667),
    f32_param!("step_size", "Step Size", 0.001, 0.02, 0.005),
    f32_param!("motion_speed", "Motion Speed", 0.0, 5.0, 1.0),
    f32_param!("brightness", "Brightness", 0.0001, 0.01, 0.001),
    f32_param!("scale", "Scale", 0.001, 0.1, 0.02),
    f32_param!("exposure", "Exposure", 0.1, 5.0, 1.0),
    f32_param!("gamma", "Gamma", 0.5, 4.0, 2.2),
    f32_param!("particle_count", "Particle Count", 100.0, 5000.0, 1000.0),
    f32_param!("dof_amount", "DOF Amount", 0.0, 1.0, 0.0),
    f32_param!("dof_focal_dist", "DOF Focal Distance", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Color 1 Red", 0.0, 1.0, 1.0),
    f32_param!("color1_g", "Color 1 Green", 0.0, 1.0, 0.5),
    f32_param!("color1_b", "Color 1 Blue", 0.0, 1.0, 0.0),
    f32_param!("color2_r", "Color 2 Red", 0.0, 1.0, 0.0),
    f32_param!("color2_g", "Color 2 Green", 0.0, 1.0, 0.5),
    f32_param!("color2_b", "Color 2 Blue", 0.0, 1.0, 1.0),
];

pub const NEBULA_PARAMS: &[BinParamSpec] = &[
    f32_param!("zoom_base", "Zoom Base", -12.0, 12.0, 0.0),
    f32_param!("space_distort_x", "Space Distort X", -0.7, 0.0, -0.2),
    f32_param!("space_distort_y", "Space Distort Y", -0.7, 0.0, -0.2),
    f32_param!("space_distort_z", "Space Distort Z", -1.5, 0.1, -0.5),
    f32_param!("zoom_delay", "Zoom Delay", 0.0, 1000.0, 0.0),
    f32_param!("zoom_speed", "Zoom Speed", 0.01, 1.0, 0.1),
    f32_param!("min_zoom", "Min Zoom", 0.1, 1.0, 0.2),
    f32_param!("noise_scale", "Noise Scale", 0.0, 200.0, 50.0),
    f32_param!("time_scale", "Time Scale", 0.0, 3.0, 1.0),
];

pub const SATAN_PARAMS: &[BinParamSpec] = &[
    f32_param!("min_radius", "Freq", 0.0, 10.0, 2.0),
    f32_param!("max_radius", "Blend", -3.0, 3.0, 0.4),
    f32_param!("size", "Darkness", -3.01, 12.2, 0.5),
    f32_param!("decay", "Vig", 0.0, 3.99, 1.0),
    color3_param!("smoke_color", "Smoke Color", 0.0, 1.0, 0.3),
    color3_param!("color2", "Color 2", 0.0, 1.0, 0.5),
];

pub const SDVERT_PARAMS: &[BinParamSpec] = &[
    f32_param!("lambda", "Vertices", 1.0, 20.0, 3.0),
    f32_param!("theta", "Angle Scale", 0.0, 10.0, 2.0),
    f32_param!("gamma", "Layer Size", 0.1, 3.0, 1.5),
    f32_param!("alpha", "Layer Min", 0.001, 0.5, 0.3),
    f32_param!("sigma", "Layer Max", 0.01, 0.5, 0.07),
    f32_param!("a", "Depth Factor", 0.0, 5.0, 2.0),
    f32_param!("b", "Fold Pattern", 0.0, 5.0, 0.5),
    f32_param!("blue", "Hue Shift", 0.0, 5.0, 1.0),
    f32_param!("base_color_r", "Base Red", 0.0, 1.0, 1.0),
    f32_param!("base_color_g", "Base Green", 0.0, 1.0, 1.0),
    f32_param!("base_color_b", "Base Blue", 0.0, 1.0, 1.0),
    f32_param!("accent_color_r", "Accent Red", 0.0, 1.0, 1.0),
    f32_param!("accent_color_g", "Accent Green", 0.0, 1.0, 1.0),
    f32_param!("accent_color_b", "Accent Blue", 0.0, 1.0, 1.0),
    f32_param!("background_r", "Background Red", 0.0, 1.0, 0.6),
    f32_param!("background_g", "Background Green", 0.0, 1.0, 0.9),
    f32_param!("background_b", "Background Blue", 0.0, 1.0, 0.9),
    f32_param!("gamma_correction", "Gamma Correction", 0.1, 3.0, 0.41),
    f32_param!("aces_tonemapping", "ACES Tonemapping", 0.0, 2.0, 0.4),
];

pub const ASAHI_PARAMS: &[BinParamSpec] = &[
    color3_param!("color_petal_start_a", "Left Start Color", 0.0, 1.0, 0.5),
    color3_param!("color_petal_end_a", "Left End Color", 0.0, 1.0, 0.0),
    color3_param!("color_petal_start_b", "Right Start Color", 0.0, 1.0, 0.0),
    color3_param!("color_petal_end_b", "Right End Color", 0.0, 1.0, 0.5),
    color3_param!("bg_color", "Background Color", 0.0, 1.0, 1.0),
    f32_param!("animation_speed", "Animation Speed", 0.0, 12.0, 4.0),
];

pub const BINS: &[&CStr] = &[
    cstr!("roto"),
    cstr!("cuneus"),
    cstr!("tree"),
    cstr!("2dneuron"),
    cstr!("gabor"),
    cstr!("plasma"),
    cstr!("lorenz"),
    cstr!("nebula"),
    cstr!("satan"),
    cstr!("sdvert"),
    cstr!("asahi"),
];

pub fn params_for_bin(bin_name: &str) -> Option<&'static [BinParamSpec]> {
    match bin_name {
        "roto" => Some(ROTO_PARAMS),
        "cuneus" => Some(CUNEUS_PARAMS),
        "spiral" => Some(SPIRAL_PARAMS),
        "voronoi" => Some(VORONOI_PARAMS),
        "matrix" => Some(MATRIX_PARAMS),
        "tree" => Some(TREE_PARAMS),
        "2dneuron" => Some(NEURON2D_PARAMS),
        "gabor" => Some(GABOR_PARAMS),
        "plasma" => Some(PLASMA_PARAMS),
        "lorenz" => Some(LORENZ_PARAMS),
        "nebula" => Some(NEBULA_PARAMS),
        "satan" => Some(SATAN_PARAMS),
        "sdvert" => Some(SDVERT_PARAMS),
        "asahi" => Some(ASAHI_PARAMS),
        _ => None,
    }
}
