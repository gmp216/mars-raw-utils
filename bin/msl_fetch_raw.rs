
use mars_raw_utils::{
    constants, 
    print, 
    vprintln, 
    jsonfetch, 
    httpfetch, 
    path, 
    util
};
use json::{JsonValue};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

#[macro_use]
extern crate clap;
use std::process;
use clap::{Arg, App};


fn print_header() {
    println!("{:37} {:15} {:6} {:20} {:27} {:6} {:6} {:7}", 
                    "ID", 
                    "Instrument",
                    "Sol",
                    "Image Date (UTC)",
                    "Image Date (Mars)",
                    "Site",
                    "Drive",
                    "Thumb"
                );
}

fn null_to_str(item:&JsonValue) -> String {
    if item.is_null() {
        return String::from("");
    } else {
        return format!("{}", item);
    }
}

fn print_image(image:&JsonValue) {
    println!("{:37} {:15} {:6} {:20} {:27} {:6} {:6} {:7}", 
                    image["imageid"], 
                    image["instrument"],
                    format!("{:6}", image["sol"]), // This is such a hack...
                    &image["date_taken"].as_str().unwrap()[..16],
                    null_to_str(&image["extended"]["lmst"]),
                    format!("{:6}", null_to_str(&image["site"])),
                    format!("{:6}", null_to_str(&image["drive"])),
                    image["is_thumbnail"]
                );
}


fn fetch_image(image:&JsonValue) {
    let image_url = &image["url"].as_str().unwrap();
    let bn = path::basename(image_url);

    // Dude, error checking!!
    let image_data = httpfetch::simple_fetch_bin(image_url).unwrap();
    
    let path = Path::new(bn.as_str());

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}", why),
        Ok(file) => file,
    };

    file.write_all(&image_data[..]).unwrap();
}

fn process_results(json_res:&JsonValue, thumbnails:bool, list_only:bool, search:&str) {

    print_header();
    vprintln!("{} images found", json_res["items"].len());
    for i in 0..json_res["items"].len() {
        let image = &json_res["items"][i];
        
        // If this image is a thumbnail and we're ignoring those, then ignore it.
        if image["is_thumbnail"].as_bool().unwrap() && ! thumbnails {
            continue;
        }

        // If we're searching for a substring and this image doesn't match, skip it.
        if search != "" && image["imageid"].as_str().unwrap().find(&search) == None {
            continue;
        }

        print_image(image);

        if !list_only {
            fetch_image(image);
        }
        
    }
}


fn main() {

    let instruments: HashMap<&str, Vec<&str>> = 
        [
            ("HAZ_FRONT", vec!["FHAZ_RIGHT_A", "FHAZ_LEFT_A", "FHAZ_RIGHT_B", "FHAZ_LEFT_B"]), 
            ("HAZ_REAR", vec!["RHAZ_RIGHT_A", "RHAZ_LEFT_A", "RHAZ_RIGHT_B", "RHAZ_LEFT_B"]), 
            ("NAV_LEFT", vec!["NAV_LEFT_A", "NAV_LEFT_B"]),
            ("NAV_RIGHT", vec!["NAV_RIGHT_A", "NAV_RIGHT_B"]),
            ("CHEMCAM", vec!["CHEMCAM_RMI"]),
            ("MARDI", vec!["MARDI"]),
            ("MAHLI", vec!["MAHLI"]),
            ("MASTCAM", vec!["MAST_LEFT", "MAST_RIGHT"])
        ].iter().cloned().collect();


    let matches = App::new(crate_name!())
                    .version(crate_version!())
                    .author(crate_authors!())
                .arg(Arg::with_name(constants::param::PARAM_VERBOSE)
                    .short(constants::param::PARAM_VERBOSE)
                    .help("Show verbose output"))
                .arg(Arg::with_name("camera")
                    .short("c")
                    .long("camera")
                    .value_name("camera")
                    .help("M20 Camera Instrument(s)")
                    .required(false)
                    .takes_value(true)
                    .multiple(true))
                .arg(Arg::with_name("sol")
                    .short("s")
                    .long("sol")
                    .value_name("sol")
                    .help("Mission Sol")
                    .required(false)
                    .takes_value(true))    
                .arg(Arg::with_name("minsol")
                    .short("m")
                    .long("minsol")
                    .value_name("minsol")
                    .help("Starting Mission Sol")
                    .required(false)
                    .takes_value(true))  
                .arg(Arg::with_name("maxsol")
                    .short("M")
                    .long("maxsol")
                    .value_name("maxsol")
                    .help("Ending Mission Sol")
                    .required(false)
                    .takes_value(true)) 
                .arg(Arg::with_name("list")
                    .short("l")
                    .long("list")
                    .value_name("list")
                    .help("Don't download, only list results")
                    .takes_value(false)
                    .required(false)) 
                .arg(Arg::with_name("thumbnails")
                    .short("t")
                    .long("thumbnails")
                    .value_name("thumbnails")
                    .help("Download thumbnails in the results")
                    .takes_value(false)
                    .required(false)) 
                .arg(Arg::with_name("num")
                    .short("n")
                    .long("num")
                    .value_name("num")
                    .help("Max number of results")
                    .required(false)
                    .takes_value(true))    
                .arg(Arg::with_name("page")
                    .short("p")
                    .long("page")
                    .value_name("page")
                    .help("Results page (starts at 1)")
                    .required(false)
                    .takes_value(true))  
                .arg(Arg::with_name("seqid")
                    .short("S")
                    .long("seqid")
                    .value_name("seqid")
                    .help("Specific sequence id or substring")
                    .required(false)
                    .takes_value(true))  
                .arg(Arg::with_name("instruments")
                    .short("i")
                    .long("instruments")
                    .value_name("instruments")
                    .help("List camera instrument and exit")
                    .takes_value(false)
                    .required(false)) 
                .get_matches();


    if matches.is_present(constants::param::PARAM_VERBOSE) {
        print::set_verbose(true);
    }

    if matches.is_present("instruments") {
        util::print_instruments(&instruments);
        process::exit(0);
    }

    let mut num_per_page = 100;
    let mut page = 1;
    let mut minsol = 1000000;
    let mut maxsol = -1;
    let mut sol = -1;
    let mut thumbnails = false;
    let mut search = "";
    let mut list_only = false;

    let mut camera_inputs: Vec<&str> = Vec::default();
    if matches.is_present("camera") {
        camera_inputs = matches.values_of("camera").unwrap().collect();
    }

    let camera_ids_res = util::find_remote_instrument_names_fromlist(&camera_inputs, &instruments);
    let cameras = match camera_ids_res {
        Err(_e) => {
            eprintln!("Invalid camera instrument(s) specified");
            process::exit(1);
        },
        Ok(v) => v,
    };


    if matches.is_present("thumbnails") {
        thumbnails = true;
    }

    if matches.is_present("list") {
        list_only = true;
    }

    if matches.is_present("seqid") {
        search =  matches.value_of("seqid").unwrap();
    }

    if matches.is_present("num") {
        let s = matches.value_of("num").unwrap();
        if util::string_is_valid_f32(&s) {
            num_per_page = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified");
            process::exit(1);
        }
    }

    if matches.is_present("page") {
        let s = matches.value_of("page").unwrap();
        if util::string_is_valid_f32(&s) {
            page = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified");
            process::exit(1);
        }
    }

    if matches.is_present("minsol") {
        let s = matches.value_of("minsol").unwrap();
        if util::string_is_valid_f32(&s) {
            minsol = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified");
            process::exit(1);
        }
    }

    if matches.is_present("maxsol") {
        let s = matches.value_of("maxsol").unwrap();
        if util::string_is_valid_f32(&s) {
            maxsol = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified");
            process::exit(1);
        }
    }

    if matches.is_present("sol") {
        let s = matches.value_of("sol").unwrap();
        if util::string_is_valid_f32(&s) {
            sol = s.parse::<i32>().unwrap();
        } else {
            eprintln!("Error: Invalid number specified");
            process::exit(1);
        }
    }

    if sol >= 0 {
        minsol = sol;
        maxsol = sol;
    }

    let joined_cameras = cameras.join("|");
    let num_per_page_s = format!("{}", num_per_page);
    let page_s = format!("{}", (page - 1));
    let minsol_s = format!("{}:sol:gte", minsol);
    let maxsol_s = format!("{}:sol:lte", maxsol);

    let params = vec![
        vec!["condition_1", "msl:mission"],
        vec!["per_page", num_per_page_s.as_str()],
        vec!["page", page_s.as_str()],
        vec!["order", "sol desc,instrument_sort asc,sample_type_sort asc, date_taken desc"],
        vec!["search", joined_cameras.as_str()],
        vec!["condition_2", minsol_s.as_str()],
        vec!["condition_3", maxsol_s.as_str()]
    ];

    let uri = constants::url::MSL_RAW_WEBSERVICE_URL;

    let mut req = jsonfetch::JsonFetcher::new(uri);

    for p in params {
        req.param(p[0], p[1]);
    }

    match req.fetch() {
        Ok(v) => process_results(&v, thumbnails, list_only, search),
        Err(_e) => eprintln!("Error fetching data from remote server")
    }

}
