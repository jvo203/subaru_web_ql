extern crate iron;
extern crate router;
extern crate mount;
extern crate staticfile;
extern crate urlencoded;
extern crate xml;

#[macro_use]
extern crate lazy_static;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use std::time::{/*Duration,*/ SystemTime};
use std::collections::HashMap;
use std::sync::Mutex;

use std::path::Path;
use staticfile::Static;
use mount::Mount;
use router::Router;

use iron::prelude::*;
use iron::status;
use urlencoded::UrlEncodedQuery;

use xml::reader::{EventReader, XmlEvent};

//static SERVER_STRING: &'static str = "SubaruWebQL v1.1.0";
//static VERSION_STRING: &'static str = "SV2018-03-19.0";

static VOTABLESERVER: &'static str = "jvox.vo.nao.ac.jp";
static VOTABLECACHE: &'static str = "VOTABLECACHE";
//static FITSCACHE: &'static str = "FITSCACHE";

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
    lock: Mutex<()>,    
    //fits FITS	 
}

lazy_static! {
    static ref HASHMAP: Mutex<HashMap<String, SubaruDataset>> = {
        let m = HashMap::new();        
        Mutex::new(m)
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

fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size).map(|_| INDENT)
             .fold(String::with_capacity(size*INDENT.len()), |r, s| r + s)
}

fn subaru_votable(subaru: &SubaruDataset, votable: &String) {
    let filename = format!("{}/{}.xml", VOTABLECACHE, subaru.data_id);
    println!("xml filename: {}", filename);

    let xmlfile = match File::open(&filename) {        
        Err(ref e) => panic!("Could not open {} ({}), downloading from {}", filename, e.description(), votable),//download VOTable
        Ok(file) => file,
    };

    let xmlfile = BufReader::new(xmlfile);
    let parser = EventReader::new(xmlfile);
    
    let mut depth = 0;
    for e in parser {
        match e {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                println!("{}+{}{:?}", indent(depth), name, attributes);
                depth += 1;
            }
            Ok(XmlEvent::Characters(text)) => {
                println!("{}{}", indent(depth), text);
            }
            Ok(XmlEvent::EndElement { name }) => {
                depth -= 1;
                println!("{}-{}", indent(depth), name);
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

fn execute_subaru(data_id: &String, votable: &String) -> IronResult<Response> {    
    println!("dataId: {}", data_id);
    println!("votable: {}", votable);

    let mut datasets = HASHMAP.lock().unwrap();
    
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
        lock: Mutex::new(()),
    });

    {
        let _guard = subaru.lock.lock().unwrap();
        subaru.timestamp = SystemTime::now() ;

        if !subaru.has_votable {
            subaru_votable(subaru, votable);
        };
    }
    
    println!("subaru: {:?}", subaru) ;
    
    Ok(Response::with((status::Ok, "SubaruWebQL request OK.")))
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
    
    let mut iron = Iron::new(mount);
    iron.threads = 4;

    println!("SubaruWebQL daemon started.");
    iron.http("localhost:8081").unwrap();
    println!("SubaruWebQL daemon ended.");//need to find a way to intercept CTRL+C as the last line does not get printed
}
