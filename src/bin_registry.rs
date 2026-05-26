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

pub const AUDIOVIS_PARAMS: &[BinParamSpec] = &[
    f32_param!("red_power", "Red Power", 0.0, 1.0, 0.5),
    f32_param!("green_power", "Green Power", 0.0, 1.0, 0.5),
    f32_param!("blue_power", "Blue Power", 0.0, 1.0, 0.5),
    f32_param!("green_boost", "Green Boost", 0.0, 1.0, 0.5),
    f32_param!("contrast", "Contrast", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
    f32_param!("glow", "Glow", 0.0, 1.0, 0.5),
];

pub const BUDDHABROT_PARAMS: &[BinParamSpec] = &[
    f32_param!("escape_radius", "Escape Radius", 0.0, 1.0, 0.5),
    f32_param!("zoom", "Zoom", 0.0, 1.0, 0.5),
    f32_param!("offset_x", "Offset X", 0.0, 1.0, 0.5),
    f32_param!("offset_y", "Offset Y", 0.0, 1.0, 0.5),
    f32_param!("rotation", "Rotation", 0.0, 1.0, 0.5),
    f32_param!("exposure", "Exposure", 0.0, 1.0, 0.5),
    f32_param!("motion_speed", "Motion Speed", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Color1 R", 0.0, 1.0, 0.5),
    f32_param!("color1_g", "Color1 G", 0.0, 1.0, 0.5),
    f32_param!("color1_b", "Color1 B", 0.0, 1.0, 0.5),
    f32_param!("color2_r", "Color2 R", 0.0, 1.0, 0.5),
    f32_param!("color2_g", "Color2 G", 0.0, 1.0, 0.5),
    f32_param!("color2_b", "Color2 B", 0.0, 1.0, 0.5),
    f32_param!("sample_density", "Sample Density", 0.0, 1.0, 0.5),
    f32_param!("dithering", "Dithering", 0.0, 1.0, 0.5),
];

pub const CLIFFORDCOMPUTE_PARAMS: &[BinParamSpec] = &[
    f32_param!("a", "A", 0.0, 1.0, 0.5),
    f32_param!("b", "B", 0.0, 1.0, 0.5),
    f32_param!("c", "C", 0.0, 1.0, 0.5),
    f32_param!("d", "D", 0.0, 1.0, 0.5),
    f32_param!("motion_speed", "Motion Speed", 0.0, 1.0, 0.5),
    f32_param!("rotation_x", "Rotation X", 0.0, 1.0, 0.5),
    f32_param!("rotation_y", "Rotation Y", 0.0, 1.0, 0.5),
    f32_param!("brightness", "Brightness", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Color1 R", 0.0, 1.0, 0.5),
    f32_param!("color1_g", "Color1 G", 0.0, 1.0, 0.5),
    f32_param!("color1_b", "Color1 B", 0.0, 1.0, 0.5),
    f32_param!("color2_r", "Color2 R", 0.0, 1.0, 0.5),
    f32_param!("color2_g", "Color2 G", 0.0, 1.0, 0.5),
    f32_param!("color2_b", "Color2 B", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
    f32_param!("dof_amount", "Dof Amount", 0.0, 1.0, 0.5),
    f32_param!("dof_focal_dist", "Dof Focal Dist", 0.0, 1.0, 0.5),
];

pub const CNN_PARAMS: &[BinParamSpec] = &[
    f32_param!("canvas_size", "Canvas Size", 0.0, 1.0, 0.5),
    f32_param!("brush_size", "Brush Size", 0.0, 1.0, 0.5),
    f32_param!("input_resolution", "Input Resolution", 0.0, 1.0, 0.5),
    f32_param!("prediction_threshold", "Prediction Threshold", 0.0, 1.0, 0.5),
    f32_param!("canvas_offset_x", "Canvas Offset X", 0.0, 1.0, 0.5),
    f32_param!("canvas_offset_y", "Canvas Offset Y", 0.0, 1.0, 0.5),
    f32_param!("feature_maps_1", "Feature Maps 1", 0.0, 1.0, 0.5),
    f32_param!("feature_maps_2", "Feature Maps 2", 0.0, 1.0, 0.5),
    f32_param!("num_classes", "Num Classes", 0.0, 1.0, 0.5),
    f32_param!("normalization_mean", "Normalization Mean", 0.0, 1.0, 0.5),
    f32_param!("normalization_std", "Normalization Std", 0.0, 1.0, 0.5),
    f32_param!("conv1_pool_size", "Conv1 Pool Size", 0.0, 1.0, 0.5),
    f32_param!("conv2_pool_size", "Conv2 Pool Size", 0.0, 1.0, 0.5),
    f32_param!("mouse_x", "Mouse X", 0.0, 1.0, 0.5),
    f32_param!("mouse_y", "Mouse Y", 0.0, 1.0, 0.5),
    f32_param!("mouse_click_x", "Mouse Click X", 0.0, 1.0, 0.5),
    f32_param!("mouse_click_y", "Mouse Click Y", 0.0, 1.0, 0.5),
];

pub const COMPUTECOLORS_PARAMS: &[BinParamSpec] = &[
    f32_param!("rotation_speed", "Rotation Speed", 0.0, 1.0, 0.5),
    f32_param!("rot_x", "Rot X", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
];

pub const DNA_PARAMS: &[BinParamSpec] = &[
    color3_param!("base_color", "Base Color", 0.0, 1.0, 0.5),
    color3_param!("rim_color", "Rim Color", 0.0, 1.0, 0.5),
    color3_param!("accent_color", "Accent Color", 0.0, 1.0, 0.5),
    f32_param!("light_intensity", "Light Intensity", 0.0, 1.0, 0.5),
    f32_param!("rim_power", "Rim Power", 0.0, 1.0, 0.5),
    f32_param!("ao_strength", "Ao Strength", 0.0, 1.0, 0.5),
    f32_param!("env_light_strength", "Env Light Strength", 0.0, 1.0, 0.5),
    f32_param!("iridescence_power", "Iridescence Power", 0.0, 1.0, 0.5),
    f32_param!("falloff_distance", "Falloff Distance", 0.0, 1.0, 0.5),
    f32_param!("vignette_strength", "Vignette Strength", 0.0, 1.0, 0.5),
    f32_param!("rotation_speed", "Rotation Speed", 0.0, 1.0, 0.5),
    f32_param!("wave_speed", "Wave Speed", 0.0, 1.0, 0.5),
    f32_param!("fold_intensity", "Fold Intensity", 0.0, 1.0, 0.5),
];

pub const DROSTE_PARAMS: &[BinParamSpec] = &[
    f32_param!("branches", "Branches", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
    f32_param!("time_scale", "Time Scale", 0.0, 1.0, 0.5),
    f32_param!("rotation", "Rotation", 0.0, 1.0, 0.5),
    f32_param!("zoom", "Zoom", 0.0, 1.0, 0.5),
    f32_param!("offset_x", "Offset X", 0.0, 1.0, 0.5),
    f32_param!("offset_y", "Offset Y", 0.0, 1.0, 0.5),
    f32_param!("iterations", "Iterations", 0.0, 1.0, 0.5),
    f32_param!("smoothing", "Smoothing", 0.0, 1.0, 0.5),
    f32_param!("use_animation", "Use Animation", 0.0, 1.0, 0.5),
];

pub const FFT_PARAMS: &[BinParamSpec] = &[
    f32_param!("filter_strength", "Filter Strength", 0.0, 1.0, 0.5),
    f32_param!("filter_direction", "Filter Direction", 0.0, 1.0, 0.5),
    f32_param!("filter_radius", "Filter Radius", 0.0, 1.0, 0.5),
];

pub const FLUID_PARAMS: &[BinParamSpec] = &[
    f32_param!("rotation_speed", "Rotation Speed", 0.0, 1.0, 0.5),
    f32_param!("motor_strength", "Motor Strength", 0.0, 1.0, 0.5),
    f32_param!("distortion", "Distortion", 0.0, 1.0, 0.5),
    f32_param!("feedback", "Feedback", 0.0, 1.0, 0.5),
];

pub const GABORNOISE_PARAMS: &[BinParamSpec] = &[
    f32_param!("width", "Width", 0.0, 1.0, 0.5),
    f32_param!("height", "Height", 0.0, 1.0, 0.5),
    f32_param!("steps", "Steps", 0.0, 1.0, 0.5),
    f32_param!("kernel_size", "Kernel Size", 0.0, 1.0, 0.5),
    f32_param!("num_kernels", "Num Kernels", 0.0, 1.0, 0.5),
    f32_param!("frequency", "Frequency", 0.0, 1.0, 0.5),
    f32_param!("frequency_var", "Frequency Var", 0.0, 1.0, 0.5),
    f32_param!("seed", "Seed", 0.0, 1.0, 0.5),
    f32_param!("animation_speed", "Animation Speed", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
];

pub const GALAXY_PARAMS: &[BinParamSpec] = &[
    f32_param!("point_intensity", "Point Intensity", 0.0, 1.0, 0.5),
    f32_param!("center_scale", "Center Scale", 0.0, 1.0, 0.5),
    f32_param!("time_scale", "Time Scale", 0.0, 1.0, 0.5),
    f32_param!("dist_offset", "Dist Offset", 0.0, 1.0, 0.5),
];

pub const GENUARY2025_6_PARAMS: &[BinParamSpec] = &[
    color3_param!("color1", "Color1", 0.0, 1.0, 0.5),
    color3_param!("gradient_color", "Gradient Color", 0.0, 1.0, 0.5),
    f32_param!("c_value_max", "C Value Max", 0.0, 1.0, 0.5),
    f32_param!("iterations", "Iterations", 0.0, 1.0, 0.5),
];

pub const HILBERT_PARAMS: &[BinParamSpec] = &[
    f32_param!("num_rays", "Num Rays", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
    f32_param!("time_scale", "Time Scale", 0.0, 1.0, 0.5),
    f32_param!("vignette_radius", "Vignette Radius", 0.0, 1.0, 0.5),
    f32_param!("vignette_softness", "Vignette Softness", 0.0, 1.0, 0.5),
    color3_param!("color_offset", "Color Offset", 0.0, 1.0, 0.5),
    f32_param!("flanc", "Flanc", 0.0, 1.0, 0.5),
];

pub const LICH_PARAMS: &[BinParamSpec] = &[
    f32_param!("cloud_density", "Cloud Density", 0.0, 1.0, 0.5),
    f32_param!("lightning_intensity", "Lightning Intensity", 0.0, 1.0, 0.5),
    f32_param!("branch_count", "Branch Count", 0.0, 1.0, 0.5),
    f32_param!("feedback_decay", "Feedback Decay", 0.0, 1.0, 0.5),
    color3_param!("base_color", "Base Color", 0.0, 1.0, 0.5),
    f32_param!("color_shift", "Color Shift", 0.0, 1.0, 0.5),
    f32_param!("spectrum_mix", "Spectrum Mix", 0.0, 1.0, 0.5),
];

pub const MANDELBULB_PARAMS: &[BinParamSpec] = &[
    f32_param!("mouse_x", "Mouse X", 0.0, 1.0, 0.5),
    f32_param!("mouse_y", "Mouse Y", 0.0, 1.0, 0.5),
    f32_param!("power", "Power", 0.0, 1.0, 0.5),
    f32_param!("animation_speed", "Animation Speed", 0.0, 1.0, 0.5),
    f32_param!("hold_duration", "Hold Duration", 0.0, 1.0, 0.5),
    f32_param!("transition_duration", "Transition Duration", 0.0, 1.0, 0.5),
    f32_param!("exposure", "Exposure", 0.0, 1.0, 0.5),
    f32_param!("focal_length", "Focal Length", 0.0, 1.0, 0.5),
    f32_param!("dof_strength", "Dof Strength", 0.0, 1.0, 0.5),
    f32_param!("palette_a_r", "Palette A R", 0.0, 1.0, 0.5),
    f32_param!("palette_a_g", "Palette A G", 0.0, 1.0, 0.5),
    f32_param!("palette_a_b", "Palette A B", 0.0, 1.0, 0.5),
    f32_param!("palette_b_r", "Palette B R", 0.0, 1.0, 0.5),
    f32_param!("palette_b_g", "Palette B G", 0.0, 1.0, 0.5),
    f32_param!("palette_b_b", "Palette B B", 0.0, 1.0, 0.5),
    f32_param!("palette_c_r", "Palette C R", 0.0, 1.0, 0.5),
    f32_param!("palette_c_g", "Palette C G", 0.0, 1.0, 0.5),
    f32_param!("palette_c_b", "Palette C B", 0.0, 1.0, 0.5),
    f32_param!("palette_d_r", "Palette D R", 0.0, 1.0, 0.5),
    f32_param!("palette_d_g", "Palette D G", 0.0, 1.0, 0.5),
    f32_param!("palette_d_b", "Palette D B", 0.0, 1.0, 0.5),
    f32_param!("manual_rotation_x", "Manual Rotation X", 0.0, 1.0, 0.5),
    f32_param!("manual_rotation_y", "Manual Rotation Y", 0.0, 1.0, 0.5),
    f32_param!("manual_rotation_z", "Manual Rotation Z", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
    f32_param!("zoom", "Zoom", 0.0, 1.0, 0.5),
    f32_param!("background_r", "Background R", 0.0, 1.0, 0.5),
    f32_param!("background_g", "Background G", 0.0, 1.0, 0.5),
    f32_param!("background_b", "Background B", 0.0, 1.0, 0.5),
    f32_param!("sun_color_r", "Sun Color R", 0.0, 1.0, 0.5),
    f32_param!("sun_color_g", "Sun Color G", 0.0, 1.0, 0.5),
    f32_param!("sun_color_b", "Sun Color B", 0.0, 1.0, 0.5),
    f32_param!("fog_color_r", "Fog Color R", 0.0, 1.0, 0.5),
    f32_param!("fog_color_g", "Fog Color G", 0.0, 1.0, 0.5),
    f32_param!("fog_color_b", "Fog Color B", 0.0, 1.0, 0.5),
    f32_param!("glow_color_r", "Glow Color R", 0.0, 1.0, 0.5),
    f32_param!("glow_color_g", "Glow Color G", 0.0, 1.0, 0.5),
    f32_param!("glow_color_b", "Glow Color B", 0.0, 1.0, 0.5),
];

pub const ORBITS_PARAMS: &[BinParamSpec] = &[
    color3_param!("base_color", "Base Color", 0.0, 1.0, 0.5),
    f32_param!("x", "X", 0.0, 1.0, 0.5),
    color3_param!("rim_color", "Rim Color", 0.0, 1.0, 0.5),
    f32_param!("y", "Y", 0.0, 1.0, 0.5),
    color3_param!("accent_color", "Accent Color", 0.0, 1.0, 0.5),
    f32_param!("gamma_correction", "Gamma Correction", 0.0, 1.0, 0.5),
    f32_param!("travel_speed", "Travel Speed", 0.0, 1.0, 0.5),
    f32_param!("col_ext", "Col Ext", 0.0, 1.0, 0.5),
    f32_param!("zoom", "Zoom", 0.0, 1.0, 0.5),
    f32_param!("trap_pow", "Trap Pow", 0.0, 1.0, 0.5),
    f32_param!("trap_x", "Trap X", 0.0, 1.0, 0.5),
    f32_param!("trap_y", "Trap Y", 0.0, 1.0, 0.5),
    f32_param!("trap_c1", "Trap C1", 0.0, 1.0, 0.5),
    f32_param!("trap_s1", "Trap S1", 0.0, 1.0, 0.5),
    f32_param!("wave_speed", "Wave Speed", 0.0, 1.0, 0.5),
    f32_param!("fold_intensity", "Fold Intensity", 0.0, 1.0, 0.5),
];

pub const PARTICLES_PARAMS: &[BinParamSpec] = &[
    f32_param!("a", "A", 0.0, 1.0, 0.5),
    f32_param!("b", "B", 0.0, 1.0, 0.5),
    f32_param!("c", "C", 0.0, 1.0, 0.5),
    f32_param!("d", "D", 0.0, 1.0, 0.5),
    f32_param!("num_circles", "Num Circles", 0.0, 1.0, 0.5),
    f32_param!("num_points", "Num Points", 0.0, 1.0, 0.5),
    f32_param!("particle_intensity", "Particle Intensity", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
    f32_param!("feedback_mix", "Feedback Mix", 0.0, 1.0, 0.5),
    f32_param!("feedback_decay", "Feedback Decay", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
    f32_param!("rotation", "Rotation", 0.0, 1.0, 0.5),
    f32_param!("bloom_scale", "Bloom Scale", 0.0, 1.0, 0.5),
    f32_param!("animation_speed", "Animation Speed", 0.0, 1.0, 0.5),
    f32_param!("color_shift_speed", "Color Shift Speed", 0.0, 1.0, 0.5),
    f32_param!("color_scale", "Color Scale", 0.0, 1.0, 0.5),
];

pub const PATHTRACING_PARAMS: &[BinParamSpec] = &[
    f32_param!("camera_pos_x", "Camera Pos X", 0.0, 1.0, 0.5),
    f32_param!("camera_pos_y", "Camera Pos Y", 0.0, 1.0, 0.5),
    f32_param!("camera_pos_z", "Camera Pos Z", 0.0, 1.0, 0.5),
    f32_param!("camera_target_x", "Camera Target X", 0.0, 1.0, 0.5),
    f32_param!("camera_target_y", "Camera Target Y", 0.0, 1.0, 0.5),
    f32_param!("camera_target_z", "Camera Target Z", 0.0, 1.0, 0.5),
    f32_param!("fov", "Fov", 0.0, 1.0, 0.5),
    f32_param!("aperture", "Aperture", 0.0, 1.0, 0.5),
    f32_param!("mouse_x", "Mouse X", 0.0, 1.0, 0.5),
    f32_param!("mouse_y", "Mouse Y", 0.0, 1.0, 0.5),
    f32_param!("rotation_speed", "Rotation Speed", 0.0, 1.0, 0.5),
    f32_param!("exposure", "Exposure", 0.0, 1.0, 0.5),
];

pub const POE2_PARAMS: &[BinParamSpec] = &[
    color3_param!("bg_color", "Bg Color", 0.0, 1.0, 0.5),
    color3_param!("hi_color", "Hi Color", 0.0, 1.0, 0.5),
    color3_param!("sh_color", "Sh Color", 0.0, 1.0, 0.5),
    color3_param!("r_base", "R Base", 0.0, 1.0, 0.5),
    color3_param!("r_dark", "R Dark", 0.0, 1.0, 0.5),
    color3_param!("m_spec", "M Spec", 0.0, 1.0, 0.5),
    f32_param!("ao_strength", "Ao Strength", 0.0, 1.0, 0.5),
    f32_param!("noise_scale", "Noise Scale", 0.0, 1.0, 0.5),
    f32_param!("wear_amount", "Wear Amount", 0.0, 1.0, 0.5),
    f32_param!("wear_scale", "Wear Scale", 0.0, 1.0, 0.5),
    f32_param!("speed", "Speed", 0.0, 1.0, 0.5),
];

pub const RORSCHACH_PARAMS: &[BinParamSpec] = &[
    f32_param!("m1_scale", "M1 Scale", 0.0, 1.0, 0.5),
    f32_param!("m1_y_scale", "M1 Y Scale", 0.0, 1.0, 0.5),
    f32_param!("m2_scale", "M2 Scale", 0.0, 1.0, 0.5),
    f32_param!("m2_shear", "M2 Shear", 0.0, 1.0, 0.5),
    f32_param!("m2_shift", "M2 Shift", 0.0, 1.0, 0.5),
    f32_param!("m3_scale", "M3 Scale", 0.0, 1.0, 0.5),
    f32_param!("m3_shear", "M3 Shear", 0.0, 1.0, 0.5),
    f32_param!("m3_shift", "M3 Shift", 0.0, 1.0, 0.5),
    f32_param!("m4_scale", "M4 Scale", 0.0, 1.0, 0.5),
    f32_param!("m4_shift", "M4 Shift", 0.0, 1.0, 0.5),
    f32_param!("m5_scale", "M5 Scale", 0.0, 1.0, 0.5),
    f32_param!("m5_shift", "M5 Shift", 0.0, 1.0, 0.5),
    f32_param!("time_scale", "Time Scale", 0.0, 1.0, 0.5),
    f32_param!("decay", "Decay", 0.0, 1.0, 0.5),
    f32_param!("intensity", "Intensity", 0.0, 1.0, 0.5),
    f32_param!("rotation_x", "Rotation X", 0.0, 1.0, 0.5),
    f32_param!("rotation_y", "Rotation Y", 0.0, 1.0, 0.5),
    f32_param!("brightness", "Brightness", 0.0, 1.0, 0.5),
    f32_param!("exposure", "Exposure", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
    f32_param!("particle_count", "Particle Count", 0.0, 1.0, 0.5),
    f32_param!("scale", "Scale", 0.0, 1.0, 0.5),
    f32_param!("dof_amount", "Dof Amount", 0.0, 1.0, 0.5),
    f32_param!("dof_focal_dist", "Dof Focal Dist", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Color1 R", 0.0, 1.0, 0.5),
    f32_param!("color1_g", "Color1 G", 0.0, 1.0, 0.5),
    f32_param!("color1_b", "Color1 B", 0.0, 1.0, 0.5),
    f32_param!("color2_r", "Color2 R", 0.0, 1.0, 0.5),
    f32_param!("color2_g", "Color2 G", 0.0, 1.0, 0.5),
    f32_param!("color2_b", "Color2 B", 0.0, 1.0, 0.5),
];

pub const SCENECOLOR_PARAMS: &[BinParamSpec] = &[
    f32_param!("num_segments", "Num Segments", 0.0, 1.0, 0.5),
    f32_param!("palette_height", "Palette Height", 0.0, 1.0, 0.5),
];

pub const SINH_PARAMS: &[BinParamSpec] = &[
    color3_param!("color1", "Color1", 0.0, 1.0, 0.5),
    color3_param!("gradient_color", "Gradient Color", 0.0, 1.0, 0.5),
    f32_param!("c_value_max", "C Value Max", 0.0, 1.0, 0.5),
];

pub const SPIRALCHAOS_PARAMS: &[BinParamSpec] = &[
    f32_param!("a", "A", 0.0, 1.0, 0.5),
    f32_param!("b", "B", 0.0, 1.0, 0.5),
    f32_param!("c", "C", 0.0, 1.0, 0.5),
    f32_param!("dof_amount", "Dof Amount", 0.0, 1.0, 0.5),
    f32_param!("dof_focal_dist", "Dof Focal Dist", 0.0, 1.0, 0.5),
    f32_param!("rotation_x", "Rotation X", 0.0, 1.0, 0.5),
    f32_param!("rotation_y", "Rotation Y", 0.0, 1.0, 0.5),
    f32_param!("brightness", "Brightness", 0.0, 1.0, 0.5),
    f32_param!("color1_r", "Color1 R", 0.0, 1.0, 0.5),
    f32_param!("color1_g", "Color1 G", 0.0, 1.0, 0.5),
    f32_param!("color1_b", "Color1 B", 0.0, 1.0, 0.5),
    f32_param!("color2_r", "Color2 R", 0.0, 1.0, 0.5),
    f32_param!("color2_g", "Color2 G", 0.0, 1.0, 0.5),
    f32_param!("color2_b", "Color2 B", 0.0, 1.0, 0.5),
];

pub const WATER_PARAMS: &[BinParamSpec] = &[
    f32_param!("camera_pos_x", "Camera Pos X", 0.0, 1.0, 0.5),
    f32_param!("camera_pos_y", "Camera Pos Y", 0.0, 1.0, 0.5),
    f32_param!("camera_pos_z", "Camera Pos Z", 0.0, 1.0, 0.5),
    f32_param!("camera_yaw", "Camera Yaw", 0.0, 1.0, 0.5),
    f32_param!("camera_pitch", "Camera Pitch", 0.0, 1.0, 0.5),
    f32_param!("water_depth", "Water Depth", 0.0, 1.0, 0.5),
    f32_param!("drag_mult", "Drag Mult", 0.0, 1.0, 0.5),
    f32_param!("camera_height", "Camera Height", 0.0, 1.0, 0.5),
    f32_param!("time_speed", "Time Speed", 0.0, 1.0, 0.5),
    f32_param!("sun_speed", "Sun Speed", 0.0, 1.0, 0.5),
    f32_param!("mouse_x", "Mouse X", 0.0, 1.0, 0.5),
    f32_param!("mouse_y", "Mouse Y", 0.0, 1.0, 0.5),
    f32_param!("atmosphere_intensity", "Atmosphere Intensity", 0.0, 1.0, 0.5),
    f32_param!("water_color_r", "Water Color R", 0.0, 1.0, 0.5),
    f32_param!("water_color_g", "Water Color G", 0.0, 1.0, 0.5),
    f32_param!("water_color_b", "Water Color B", 0.0, 1.0, 0.5),
    f32_param!("sun_color_r", "Sun Color R", 0.0, 1.0, 0.5),
    f32_param!("sun_color_g", "Sun Color G", 0.0, 1.0, 0.5),
    f32_param!("sun_color_b", "Sun Color B", 0.0, 1.0, 0.5),
    f32_param!("cloud_coverage", "Cloud Coverage", 0.0, 1.0, 0.5),
    f32_param!("cloud_speed", "Cloud Speed", 0.0, 1.0, 0.5),
    f32_param!("cloud_height", "Cloud Height", 0.0, 1.0, 0.5),
    f32_param!("night_sky_r", "Night Sky R", 0.0, 1.0, 0.5),
    f32_param!("night_sky_g", "Night Sky G", 0.0, 1.0, 0.5),
    f32_param!("night_sky_b", "Night Sky B", 0.0, 1.0, 0.5),
    f32_param!("exposure", "Exposure", 0.0, 1.0, 0.5),
    f32_param!("gamma", "Gamma", 0.0, 1.0, 0.5),
    f32_param!("fresnel_strength", "Fresnel Strength", 0.0, 1.0, 0.5),
    f32_param!("reflection_strength", "Reflection Strength", 0.0, 1.0, 0.5),
];

pub const BINS: &[&CStr] = &[
    cstr!("roto"),
    cstr!("cuneus"),
    cstr!("spiral"),
    cstr!("voronoi"),
    cstr!("matrix"),
    cstr!("tree"),
    cstr!("2dneuron"),
    cstr!("gabor"),
    cstr!("plasma"),
    cstr!("lorenz"),
    cstr!("nebula"),
    cstr!("satan"),
    cstr!("sdvert"),
    cstr!("asahi"),
    cstr!("audiovis"),
    cstr!("buddhabrot"),
    cstr!("cliffordcompute"),
    cstr!("cnn"),
    cstr!("computecolors"),
    cstr!("dna"),
    cstr!("droste"),
    cstr!("fft"),
    cstr!("fluid"),
    cstr!("gabornoise"),
    cstr!("galaxy"),
    cstr!("genuary2025_6"),
    cstr!("hilbert"),
    cstr!("lich"),
    cstr!("mandelbulb"),
    cstr!("orbits"),
    cstr!("particles"),
    cstr!("pathtracing"),
    cstr!("poe2"),
    cstr!("rorschach"),
    cstr!("scenecolor"),
    cstr!("sinh"),
    cstr!("spiralchaos"),
    cstr!("water"),
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
        "audiovis" => Some(AUDIOVIS_PARAMS),
        "buddhabrot" => Some(BUDDHABROT_PARAMS),
        "cliffordcompute" => Some(CLIFFORDCOMPUTE_PARAMS),
        "cnn" => Some(CNN_PARAMS),
        "computecolors" => Some(COMPUTECOLORS_PARAMS),
        "dna" => Some(DNA_PARAMS),
        "droste" => Some(DROSTE_PARAMS),
        "fft" => Some(FFT_PARAMS),
        "fluid" => Some(FLUID_PARAMS),
        "gabornoise" => Some(GABORNOISE_PARAMS),
        "galaxy" => Some(GALAXY_PARAMS),
        "genuary2025_6" => Some(GENUARY2025_6_PARAMS),
        "hilbert" => Some(HILBERT_PARAMS),
        "lich" => Some(LICH_PARAMS),
        "mandelbulb" => Some(MANDELBULB_PARAMS),
        "orbits" => Some(ORBITS_PARAMS),
        "particles" => Some(PARTICLES_PARAMS),
        "pathtracing" => Some(PATHTRACING_PARAMS),
        "poe2" => Some(POE2_PARAMS),
        "rorschach" => Some(RORSCHACH_PARAMS),
        "scenecolor" => Some(SCENECOLOR_PARAMS),
        "sinh" => Some(SINH_PARAMS),
        "spiralchaos" => Some(SPIRALCHAOS_PARAMS),
        "water" => Some(WATER_PARAMS),
        _ => None,
    }
}
