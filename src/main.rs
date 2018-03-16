extern crate iron;
extern crate router;
extern crate mount;
extern crate staticfile;
extern crate urlencoded;

use std::path::Path;
use staticfile::Static;
use mount::Mount;
use router::Router;

use iron::prelude::*;
use iron::status;
use urlencoded::UrlEncodedQuery;

fn subaru_handler(req: &mut Request) -> IronResult<Response> {    
    println!("SubaruWebQL.html request handler");    

    match req.get_ref::<UrlEncodedQuery>() {
        Ok(ref hashmap) => println!("Parsed GET request query string:\n {:?}", hashmap),
        Err(ref e) => println!("{:?}", e)
    };
    
    Ok(Response::with((status::Ok, "SubaruWebQL request was routed!")))
}

//static SERVER_STRING: &'static str ="SubaruWebQL v1.1.0"
//const VERSION_STRING = "SV2018-03-14.0"

fn main() {    
    /*let mut iron = Iron::new(|_: &mut Request| {
        Ok(Response::with((status::Ok, "SubaruWebQL")))
    });*/

    let mut router = Router::new();    
    router.get("/", subaru_handler, "votable");
    
    let mut mount = Mount::new();    
    mount.mount("/subaruwebql/SubaruWebQL.html", router);    
    mount.mount("/", Static::new(Path::new("target/htdocs/subaru.html")));
    mount.mount("/favicon.ico", Static::new(Path::new("target/htdocs/favicon.ico")));
    mount.mount("/subaruwebql/", Static::new(Path::new("target/htdocs/subaruwebql/")));
    
    let mut iron = Iron::new(mount);
    iron.threads = 4;

    println!("SubaruWebQL daemon started.");
    iron.http("localhost:8081").unwrap();
    println!("SubaruWebQL daemon ended.");//need to find a way to intercept CTRL+C as the last line does not get printed
}
