use mars_raw_utils::{
    constants, 
    print, 
    vprintln, 
    rgbimage, 
    enums, 
    path,
    util,
    decompanding
};

#[macro_use]
extern crate clap;

use std::process;

use clap::{Arg, App};



fn process_file(input_file:&str, red_scalar:f32, green_scalar:f32, blue_scalar:f32, color_noise_reduction:i32, no_ilt:bool) {
    
    let instrument = enums::Instrument::MslMastcamLeft;

    let mut raw = rgbimage::RgbImage::open(input_file, instrument).unwrap();

    let mut data_max = 255.0;

    if ! no_ilt {
        vprintln!("Decompanding...");
        raw.decompand().unwrap();
        data_max = decompanding::get_max_for_instrument(instrument) as f32;
    }

    vprintln!("Debayering...");
    raw.debayer().unwrap();
    
    vprintln!("Inpainting...");
    raw.apply_inpaint_fix().unwrap();

    //vprintln!("Flatfielding...");
    //raw.flatfield(enums::Instrument::MslMAHLI).unwrap();

    vprintln!("Applying color weights...");
    raw.apply_weight(red_scalar, green_scalar, blue_scalar).unwrap();

    if color_noise_reduction > 0 {
        vprintln!("Color noise reduction...");
        raw.reduce_color_noise(color_noise_reduction).unwrap();
    }
    
    vprintln!("Normalizing...");
    raw.normalize_to_16bit_with_max(data_max).unwrap();

    vprintln!("Writing to disk...");

    let out_file = input_file.replace(".jpg", "-rjcal.png").replace(".JPG", "-rjcal.png");
    raw.save(&out_file).unwrap();
}



fn main() {
    
    let matches = App::new(crate_name!())
                    .version(crate_version!())
                    .author(crate_authors!())
                    .arg(Arg::with_name(constants::param::PARAM_INPUTS)
                        .short(constants::param::PARAM_INPUTS_SHORT)
                        .long(constants::param::PARAM_INPUTS)
                        .value_name("INPUT")
                        .help("Input")
                        .required(true)
                        .multiple(true)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_RED_WEIGHT)
                        .short(constants::param::PARAM_RED_WEIGHT_SHORT)
                        .long(constants::param::PARAM_RED_WEIGHT)
                        .value_name("RED")
                        .help("Red weight")
                        .required(false)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_GREEN_WEIGHT)
                        .short(constants::param::PARAM_GREEN_WEIGHT_SHORT)
                        .long(constants::param::PARAM_GREEN_WEIGHT)
                        .value_name("GREEN")
                        .help("Green weight")
                        .required(false)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_BLUE_WEIGHT)
                        .short(constants::param::PARAM_BLUE_WEIGHT_SHORT)
                        .long(constants::param::PARAM_BLUE_WEIGHT)
                        .value_name("BLUE")
                        .help("Blue weight")
                        .required(false)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_COLOR_NOISE_REDUCTION)
                        .short(constants::param::PARAM_COLOR_NOISE_REDUCTION_SHORT)
                        .long(constants::param::PARAM_COLOR_NOISE_REDUCTION)
                        .value_name("COLOR_NOISE_REDUCTION")
                        .help("Color noise reduction amount in pixels")
                        .required(false)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_VERBOSE)
                        .short(constants::param::PARAM_VERBOSE)
                        .help("Show verbose output"))
                    .arg(Arg::with_name(constants::param::PARAM_RAW_COLOR)
                        .short(constants::param::PARAM_RAW_COLOR_SHORT)
                        .long(constants::param::PARAM_RAW_COLOR)
                        .help("Raw color, skip ILT"))
                    .get_matches();

    if matches.is_present(constants::param::PARAM_VERBOSE) {
        print::set_verbose(true);
    }

    let mut red_scalar = constants::DEFAULT_RED_WEIGHT;
    let mut green_scalar = constants::DEFAULT_GREEN_WEIGHT;
    let mut blue_scalar = constants::DEFAULT_BLUE_WEIGHT;
    let mut color_noise_reduction = 0;
    let mut no_ilt = false;
    
    if matches.is_present(constants::param::PARAM_RAW_COLOR) {
        no_ilt = true;
    }

    // Check formatting and handle it
    if matches.is_present(constants::param::PARAM_RED_WEIGHT) {
        let s = matches.value_of(constants::param::PARAM_RED_WEIGHT).unwrap();
        if util::string_is_valid_f32(&s) {
            red_scalar = s.parse::<f32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified for red scalar");
            process::exit(1);
        }
    }

    if matches.is_present(constants::param::PARAM_GREEN_WEIGHT) {
        let s = matches.value_of(constants::param::PARAM_GREEN_WEIGHT).unwrap();
        if util::string_is_valid_f32(&s) {
            green_scalar = s.parse::<f32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified for green scalar");
            process::exit(1);
        }
    }

    if matches.is_present(constants::param::PARAM_BLUE_WEIGHT) {
        let s = matches.value_of(constants::param::PARAM_BLUE_WEIGHT).unwrap();
        if util::string_is_valid_f32(&s) {
            blue_scalar = s.parse::<f32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified for blue scalar");
            process::exit(1);
        }
    }

    if matches.is_present(constants::param::PARAM_COLOR_NOISE_REDUCTION) {
        let s = matches.value_of(constants::param::PARAM_COLOR_NOISE_REDUCTION).unwrap();
        if util::string_is_valid_i32(&s) {
            color_noise_reduction = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified for color noise reduction");
            process::exit(1);
        }
        if color_noise_reduction % 2 == 0 {
            eprintln!("Error: Color noise reduction value must be odd");
            process::exit(1);
        }
        if color_noise_reduction < 0 {
            eprintln!("Error: Color noise reduction value must a positive number");
            process::exit(1);
        }
    }

    let input_files: Vec<&str> = matches.values_of(constants::param::PARAM_INPUTS).unwrap().collect();

    for in_file in input_files.iter() {
        if path::file_exists(in_file) {
            vprintln!("Processing File: {}", in_file);
            process_file(in_file, red_scalar, green_scalar, blue_scalar, color_noise_reduction, no_ilt);
        } else {
            eprintln!("File not found: {}", in_file);
        }
    }

    
}
