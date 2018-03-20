extern crate iron;
extern crate router;
extern crate mount;
extern crate staticfile;
extern crate urlencoded;
extern crate xml;
extern crate curl;

#[macro_use] extern crate scan_fmt;

#[macro_use]
extern crate lazy_static;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use std::thread;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;
//use std::sync::Mutex;
use std::sync::RwLock;

use std::path::Path;
use staticfile::Static;
use mount::Mount;
use router::Router;

use iron::prelude::*;
use iron::status;
use urlencoded::UrlEncodedQuery;
use iron::modifiers::Header;
use iron::headers::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel};

use curl::easy::Easy;
use std::io::Write;
use xml::reader::{EventReader, XmlEvent};

//static SERVER_STRING: &'static str = "SubaruWebQL v1.1.0";
static VERSION_STRING: &'static str = "SV2018-03-19.0";

static VOTABLESERVER: &'static str = "jvox.vo.nao.ac.jp";
static VOTABLECACHE: &'static str = "VOTABLECACHE";
static FITSCACHE: &'static str = "FITSCACHE";

#[derive(Debug)]
struct SubaruDataset {	
    data_id: String,
    process_id: String,
    title: String,
    date_obs: String,
    objects: String,
    band_name: String,
    band_ref: String,
    band_hi: String,
    band_lo: String,
    band_unit: String,
    ra: String,
    dec: String,
    file_size: u64,
    file_path: String,
    file_url: String,
    current_pos: i32,
    data_id_pos: i32,
    process_id_pos: i32,
    title_pos: i32,
    date_obs_pos: i32,
    objects_pos: i32,
    band_name_pos: i32,
    band_ref_pos: i32,
    band_hi_pos: i32,
    band_lo_pos: i32,
    band_unit_pos: i32,
    ra_pos: i32,
    dec_pos: i32,
    file_size_pos: i32,
    file_path_pos: i32,
    file_url_pos: i32,
    timestamp: SystemTime,
    has_votable: bool,
    has_fits: bool,
    //lock: Mutex<()>,    
    //fits FITS	 
}

lazy_static! {
    static ref HASHMAP: RwLock<HashMap<String, SubaruDataset>> = {
        let m = HashMap::new();        
        RwLock::new(m)
    };
}

fn subaru_handler(req: &mut Request) -> IronResult<Response> {    
    println!("SubaruWebQL.html request handler");    

    let hashmap = match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => {
            println!("Parsed GET request query string:\n {:?}", hashmap);
            *hashmap
        },
        Err(ref e) => {
            println!("{:?}", e);
            return Ok(Response::with((status::NotFound, format!("SubaruWebQL request error: {:?}", e))));
        }
    };

    let data_id = match hashmap.get("dataId") {
        Some(x) => {
            let s = &x[0];
            
            if s.is_empty() {
                println!("empty dataId found!");
                return Ok(Response::with((status::NotFound, ("SubaruWebQL request error: empty dataId found!"))));
            } else {
                s//return the first value from the params value vector
            }
        },
        None => {
            println!("no dataId found!");
            return Ok(Response::with((status::NotFound, ("SubaruWebQL request error: dataId not found!"))));
        }
    };
    //we've got a non-empty dataId    
    
    let votable = match hashmap.get("votable") {
        Some(x) => {
            (&x[0]).clone()//copy the first value from the params value vector
        },
        None => {
            println!("no votable found, defaulting to Suprime-Cam.");
            //return a default Suprime-Cam votable for a given dataId
            format!("http://{}:8060/skynode/do/tap/spcam/sync?REQUEST=queryData&QUERY=SELECT%20*%20FROM%20image_nocut%20WHERE%20data_id%20='{}'", VOTABLESERVER, data_id)            
        }
    };
    //we've got votable too

    execute_subaru(&data_id, &votable)
    //Ok(Response::with((status::Ok, format!("SubaruWebQL request OK.\n{:?}", hashmap))))
}

fn subaru_votable(subaru: &mut SubaruDataset, votable: &String) {
    let filename = format!("{}/{}.xml", VOTABLECACHE, subaru.data_id);
    println!("xml filename: {}", filename);

    let xmlfile = match File::open(&filename) {        
        Err(ref _e) => {
            let mut easy = Easy::new();

            easy.url(votable).unwrap();                       
            
            let tmp = format!("{}/{}.xml.tmp", VOTABLECACHE, subaru.data_id);
            let mut tmpfile = match File::create(&tmp) {
                Err(ref e) => panic!("Could not open {} ({})!", tmp, e.description()),                    
                Ok(file) => file,
            };
            
            let mut transfer = easy.transfer();           

            transfer.write_function(|data| {                    
                tmpfile.write_all(data).unwrap();
                Ok(data.len())
            }).unwrap(); 
            
            transfer.perform().unwrap();                       
            
            std::fs::rename(tmp, filename.clone()).unwrap();
            File::open(&filename).unwrap()
        },
        Ok(file) => file,
    };

    let xmlfile = BufReader::new(xmlfile);
    let parser = EventReader::new(xmlfile);
    
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                if name.local_name == "FIELD" {
                    for attr in attributes.iter() {
                        if attr.name.local_name.trim() == "ID" {
                            let current_pos = scan_fmt!(&attr.value, "C{d}", i32);

                            match current_pos {
                                Some(current_pos) => subaru.current_pos = current_pos,
                                _ => println!("ID scan_fmt! error")                                
                            }
                        }

                        if attr.name.local_name.trim() == "name" {
                            match attr.value.trim() {                            
                                "DATA_ID" => subaru.data_id_pos = subaru.current_pos,
                                "PROC_ID"=>	subaru.process_id_pos = subaru.current_pos,
			        "TITLE" => subaru.title_pos = subaru.current_pos,
			        "DATE_OBS" => subaru.date_obs_pos = subaru.current_pos,
			        "OBJECTS" => subaru.objects_pos = subaru.current_pos,
			        "BAND_NAME" => subaru.band_name_pos = subaru.current_pos,
			        "BAND_REFVAL" => subaru.band_ref_pos = subaru.current_pos,
			        "BAND_HILIMIT" => subaru.band_hi_pos = subaru.current_pos,
			        "BAND_LOLIMIT" => subaru.band_lo_pos = subaru.current_pos,
			        "BAND_UNIT" => subaru.band_unit_pos = subaru.current_pos,
			        "CENTER_RA" => subaru.ra_pos = subaru.current_pos,
			        "CENTER_DEC" => subaru.dec_pos = subaru.current_pos,
			        "FILE_SIZE" => subaru.file_size_pos = subaru.current_pos,
			        "PATH" => subaru.file_path_pos = subaru.current_pos,
			        "ACCESS_REF" => subaru.file_url_pos = subaru.current_pos,
                                _ => (),
                            }
                        }
                    }
                }//end-of-FIELD

                if name.local_name == "FIELD" {
                    subaru.current_pos = 0;
                }//end-of-"TR"

                if name.local_name == "TD" {
		    subaru.current_pos += 1;
                }//end-of-"TR"                
            }
            Ok(XmlEvent::Characters(text)) => {
                if subaru.current_pos == subaru.process_id_pos {
                    subaru.process_id = text.clone();
                }

                if subaru.current_pos == subaru.title_pos {
		    subaru.title = text.clone();
                }

                if subaru.current_pos == subaru.date_obs_pos {
		    subaru.date_obs = text.clone();
                }

                if subaru.current_pos == subaru.date_obs_pos {
		    subaru.date_obs = text.clone();
                }

                if subaru.current_pos == subaru.objects_pos {
		    subaru.objects = text.clone();
                }

                if subaru.current_pos == subaru.band_name_pos {
		    subaru.band_name = text.clone();
                }

                if subaru.current_pos == subaru.band_ref_pos {
		    subaru.band_ref = text.clone();
                }

                if subaru.current_pos == subaru.band_hi_pos {
		    subaru.band_hi = text.clone();
                }

                if subaru.current_pos == subaru.band_lo_pos {
		    subaru.band_lo = text.clone();
                }

                if subaru.current_pos == subaru.band_unit_pos {		    
                    subaru.band_unit = text.clone();
                    
		    if subaru.band_unit == "A" {
			subaru.band_unit = String::from("&#8491;");
		    }
		    
		    if subaru.band_unit == "um" {
			subaru.band_unit = String::from("&#181;m");
		    }
                }

                if subaru.current_pos == subaru.ra_pos {
		    subaru.ra = text.clone();
                }

                if subaru.current_pos == subaru.dec_pos {
		    subaru.dec = text.clone();
                }

                if subaru.current_pos == subaru.file_size_pos {
                    let file_size = scan_fmt!(&text, "{d}", u64);

                    match file_size {
                        Some(file_size) => subaru.file_size = file_size,
                        _ => println!("file_size scan_fmt! error")                                
                    }
                }

                if subaru.current_pos == subaru.file_path_pos {
		    subaru.file_path = text.clone();
                }

                if subaru.current_pos == subaru.file_url_pos {
		    subaru.file_url = text.clone();
                }
            }            
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    subaru.has_votable = true;
}

fn subaru_fits(subaru: &mut SubaruDataset) {
    let filename = format!("{}/{}.fits", FITSCACHE, subaru.data_id);
    println!("FITS filename: {}", filename);

    subaru.has_fits = true;
}

/*fn execute_subaru(data_id: &String, votable: &String) -> IronResult<Response> {    
println!("dataId: {}", data_id);
println!("votable: {}", votable);

let datasets = HASHMAP.read().unwrap();

if !datasets.contains_key(data_id) {
//a stall on the mutex (previous read lock prevents a write lock
let mut datasets = HASHMAP.write().unwrap();        

let subaru = datasets.entry(data_id.clone()).or_insert(SubaruDataset {
data_id: data_id.clone(),
process_id: String::from(""),
title: String::from(""),
date_obs: String::from(""),
objects: String::from(""),
band_name: String::from(""),
band_ref: String::from(""),
band_hi: String::from(""),
band_lo: String::from(""),
band_unit: String::from(""),
ra: String::from(""),
dec: String::from(""),
file_size: 0,
file_path: String::from(""),
file_url: String::from(""),
current_pos: -1,
data_id_pos: -1,
process_id_pos: -1,
title_pos: -1,
date_obs_pos: -1,
objects_pos: -1,
band_name_pos: -1,
band_ref_pos: -1,
band_hi_pos: -1,
band_lo_pos: -1,
band_unit_pos: -1,
ra_pos: -1,
dec_pos: -1,
file_size_pos: -1,
file_path_pos: -1,
file_url_pos: -1,
timestamp: SystemTime::now(),
has_votable: false,
//lock: Mutex::new(()),
        });

        if !subaru.has_votable {
            subaru_votable(subaru, votable);
        };
    };

    let subaru = datasets.get(data_id).unwrap();    
    
    println!("subaru: {:?}", subaru) ;
    
    Ok(Response::with((status::Ok, "SubaruWebQL request OK.")))
}*/

fn execute_subaru(data_id: &String, votable: &String) -> IronResult<Response> {    
    println!("dataId: {}", data_id);
    println!("votable: {}", votable);
    
    let mut datasets = HASHMAP.write().unwrap();    
    
    let subaru = datasets.entry(data_id.clone()).or_insert(SubaruDataset {
        data_id: data_id.clone(),
        process_id: String::from(""),
        title: String::from(""),
        date_obs: String::from(""),
        objects: String::from(""),
        band_name: String::from(""),
        band_ref: String::from(""),
        band_hi: String::from(""),
        band_lo: String::from(""),
        band_unit: String::from(""),
        ra: String::from(""),
        dec: String::from(""),
        file_size: 0,
        file_path: String::from(""),
        file_url: String::from(""),
        current_pos: -1,
        data_id_pos: -1,
        process_id_pos: -1,
        title_pos: -1,
        date_obs_pos: -1,
        objects_pos: -1,
        band_name_pos: -1,
        band_ref_pos: -1,
        band_hi_pos: -1,
        band_lo_pos: -1,
        band_unit_pos: -1,
        ra_pos: -1,
        dec_pos: -1,
        file_size_pos: -1,
        file_path_pos: -1,
        file_url_pos: -1,
        timestamp: SystemTime::now(),
        has_votable: false,
        has_fits: false,
        //lock: Mutex::new(()),
    });
    
    //let _guard = subaru.lock.lock().unwrap();
    subaru.timestamp = SystemTime::now() ;

    if !subaru.has_votable {
        subaru_votable(subaru, votable);
    }        
    
    println!("subaru: {:?}", subaru) ;

    let mut html = String::from("<!DOCTYPE html>\n<html xmlns:xlink=\"http://www.w3.org/1999/xlink\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<link rel=\"stylesheet\" type=\"text/css\" href=\"http://fonts.googleapis.com/css?family=Inconsolata\">\n");
    html.push_str("<script src=\"https://d3js.org/d3.v4.min.js\"></script>\n");
    html.push_str("<script src=\"/subaruwebql/progressbar.min.js\"></script>\n");
    html.push_str("<script src=\"/subaruwebql/ra_dec_conversion.js\"></script>\n");
    html.push_str("<script src=\"/subaruwebql/reconnecting-websocket.min.js\"></script>\n");
    html.push_str("<script src=\"/subaruwebql/subaruwebql.js\"></script>\n");
    html.push_str("<!-- Latest compiled and minified CSS --> <link rel=\"stylesheet\" href=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap.min.css\" integrity=\"sha384-BVYiiSIFeK1dGmJRAkycuHAHRg32OmUcww7on3RYdg4Va+PmSTsz/K68vbdEjh4u\" crossorigin=\"anonymous\">\n");
    html.push_str("<!-- Optional theme --> <link rel=\"stylesheet\" href=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap-theme.min.css\" integrity=\"sha384-rHyoN1iRsVXV4nD0JutlnGaslCJuC7uwjduW9SVrLvRYooPp2bWYgmgJQIXwl/Sp\" crossorigin=\"anonymous\">\n");
    html.push_str("<!-- jQuery (necessary for Bootstrap's JavaScript plugins) --> <script src=\"https://ajax.googleapis.com/ajax/libs/jquery/1.12.4/jquery.min.js\"></script>\n");
    html.push_str("<!-- Latest compiled and minified JavaScript --> <script src=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/js/bootstrap.min.js\" integrity=\"sha384-Tc5IQib027qvyjSMfHjOMaLkfuWVxZxUPnCJA7l2mCWNIpG9mGCD8wGNIcPD7Txa\" crossorigin=\"anonymous\"></script>\n");
    html.push_str("<link rel=\"stylesheet\" href=\"/subaruwebql/subaruwebql.css\"/>\n");
    html.push_str("<script src=\"/subaruwebql/lz4.min.js\" charset=\"utf-8\"></script>\n");
    html.push_str("<title>SubaruWebQL</title></head><body>\n");

    html.push_str(&format!("<div id='votable' style='width: 0; height: 0;' data-dataId='{}' data-processId='{}' data-title='{}' data-date='{}' data-objects='{}' data-band-name='{}' data-band-ref='{}' data-band-hi='{}' data-band-lo='{}' data-band-unit='{}' data-ra='{}' data-dec='{}' data-filesize='{}' data-server-version='{}'></div>\n", data_id, subaru.process_id, subaru.title, subaru.date_obs, subaru.objects, subaru.band_name, subaru.band_ref, subaru.band_hi, subaru.band_lo, subaru.band_unit, subaru.ra, subaru.dec, subaru.file_size, VERSION_STRING));

    html.push_str("<script>
const golden_ratio = 1.6180339887;
var firstTime = true ;
mainRenderer();
window.onresize = resize;
function resize(){mainRenderer();}
  </script>");
    
    html.push_str("</body></html>");

    if !subaru.has_fits {
        /*thread::spawn(move || {
            subaru_fits(subaru);
        });*/

        subaru_fits(subaru);
    }

    let content_type = Header(ContentType(Mime(TopLevel::Text, SubLevel::Html, vec![])));
    Ok(Response::with((status::Ok, html, content_type)))
}

fn main() {    
    /*let mut iron = Iron::new(|_: &mut Request| {
        Ok(Response::with((status::Ok, "SubaruWebQL")))
    });*/

    let mut router = Router::new();    
    router.get("/", subaru_handler, "votable");
    
    let mut mount = Mount::new();    
    mount.mount("/subaruwebql/SubaruWebQL.html", router);    
    mount.mount("/", Static::new(Path::new("htdocs/subaru.html")));
    mount.mount("/favicon.ico", Static::new(Path::new("htdocs/favicon.ico")));
    mount.mount("/subaruwebql/", Static::new(Path::new("htdocs/subaruwebql/")));
    
    let iron = Iron::new(mount);
    //iron.threads = 4;//using threads is very slow!!!

    println!("SubaruWebQL daemon started.");
    iron.http("localhost:8081").unwrap();
    println!("SubaruWebQL daemon ended.");//need to find a way to intercept CTRL+C as the last line does not get printed
}
