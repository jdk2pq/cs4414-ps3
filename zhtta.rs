//
// zhtta.rs
//
// Running on Rust 0.8
//
// Starting code for PS3
// 
// Note: it would be very unwise to run this server on a machine that is
// on the Internet and contains any sensitive files!
//
// University of Virginia - cs4414 Fall 2013
// Weilin Xu and David Evans
// Version 0.3

extern mod extra;


use std::rt::io::*;
use std::rt::io::net::ip::SocketAddr;
use std::io::println;
use std::cell::Cell;
use std::{os, str, io};
use extra::arc;
use std::comm::*;


static PORT:    int = 4414;
static IP: &'static str = "127.0.0.1";


struct sched_msg {
    stream: Option<std::rt::io::net::tcp::TcpStream>,
    filepath: ~std::path::PosixPath
}

fn main() {
    let req_vec: ~[sched_msg] = ~[];
    let shared_req_vec = arc::RWArc::new(req_vec);
    let add_vec = shared_req_vec.clone();
    let take_vec = shared_req_vec.clone();
    
    let (port, chan) = stream();
    let chan = SharedChan::new(chan);

    let visitor_count: uint = 0;
    let shared_visitor_count = arc::RWArc::new(visitor_count);

    //Wahoo Scheduling
    let wahoo_index:uint = 0;
    let shared_wahoo_index = arc::RWArc::new(wahoo_index);
    let local_wahoo_index = shared_wahoo_index.clone();


    // dequeue file requests, and send responses.
    // FIFO
    do spawn {
        let (sm_port, sm_chan) = stream();

        // a task for sending responses.
        do spawn {
            loop {
                let mut tf: sched_msg = sm_port.recv(); // wait for the dequeued request to handle
                match io::read_whole_file(tf.filepath) { // killed if file size is larger than memory size.
                    Ok(file_data) => {
                        println(fmt!("begin serving file [%?]", tf.filepath));
                        // A web server should always reply a HTTP header for any legal HTTP request.
                        tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                        tf.stream.write(file_data);
                        println(fmt!("finish file [%?]", tf.filepath));
                    }
                    Err(err) => {
                        println(err);
                    }
                }
            }
        }
        
        loop {
            port.recv(); // wait for arrving notification
            do take_vec.write |vec| {
                if ((*vec).len() > 0) {
                    // LIFO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                    /*
                    let tf_opt: Option<sched_msg> = (*vec).shift_opt();
                    let tf = tf_opt.unwrap();
                    println(fmt!("shift from queue, size: %ud", (*vec).len()));
                    sm_chan.send(tf); // send the request to send-response-task to serve.
                    */
                    //Find shortest process
                    let mut wahoo_index = 0;
                    let mut min_index: uint = 0;      let mut min_size: int = 0;
                    do local_wahoo_index.write |count| { wahoo_index = (*count) }
                    
                    //Find shortest Charlottesville request, else do other requests
                    if wahoo_index > 0 {
                        for i in range(0, wahoo_index) {
                            let size =  (*vec)[i].filepath.get_size().unwrap_or_zero();
                            if min_size <= 0 || min_size > size as int{
                                min_size = size as int;
                                min_index = i;
                            }
                        }
                    }
                    else {
                        for i in range(0, (*vec).len()) {
                            let size =  (*vec)[i].filepath.get_size().unwrap_or_zero();
                            if min_size <= 0 || min_size > size as int {
                                min_size = size as int;
                                min_index = i;
                            }
                        }
                    }

                    let tf = (*vec).remove(min_index);
                    println(fmt!("shift from queue, size: %ud", (*vec).len()));
                    sm_chan.send(tf); // send the request to send-response-task to serve.

                    //Decrement wahoo index
                    do local_wahoo_index.write |count| {
                        if (wahoo_index > 0) {
                            (*count) -= 1;
                            println(fmt!("wahoo index decremented to %u", (*count)));
                        }
                    }                  
                }
            }
        }
    }

    let ip = match FromStr::from_str(IP) { Some(ip) => ip, 
                                           None => { println(fmt!("Error: Invalid IP address <%s>", IP));
                                                     return;},
                                         };
                                         
    let socket = net::tcp::TcpListener::bind(SocketAddr {ip: ip, port: PORT as u16});
    
    println(fmt!("Listening on %s:%d ...", ip.to_str(), PORT));
    let mut acceptor = socket.listen().unwrap();
    
    for stream in acceptor.incoming() {
        //Getting the ip address of the socket
        let mut stream = stream;
        let mut incoming_ip: ~str = ~"";
        match stream {
            Some(ref mut theTcpStream) => {
                match theTcpStream.peer_name() {
                    Some(peername) => {
                        incoming_ip = peername.to_str();
                    }
                    None => ()                    
                }
            }
            None => ()
        }

        //Cells to pass in the strem and ip address to the closure function
        let stream = Cell::new(stream);
        let incoming_ip = Cell::new(incoming_ip);

        let local_visitor_count = shared_visitor_count.clone();
        let local_wahoo_index = shared_wahoo_index.clone();
        // Start a new task to handle the each connection
        let child_chan = chan.clone();
        let child_add_vec = add_vec.clone();

        do spawn {
            let mut visitor_count = 0;
            do local_visitor_count.write |count| {
                (*count) += 1;
                visitor_count = (*count);
            }
            
            let mut stream = stream.take();
            let mut incoming_ip = incoming_ip.take();

            let mut buf = [0, ..500];
            stream.read(buf);
            let request_str = str::from_utf8(buf);
            
            let req_group : ~[&str]= request_str.splitn_iter(' ', 3).collect();
            if req_group.len() > 2 {
                let path = req_group[1];
                println(fmt!("Request for path: \n%?", path));
                
                let file_path = ~os::getcwd().push(path.replace("/../", ""));
                if !os::path_exists(file_path) || os::path_is_dir(file_path) {
                    println(fmt!("Request received:\n%s", request_str));
                    let response: ~str = fmt!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n
                         <doctype !html><html><head><title>Hello, Rust!</title>
                         <style>body { background-color: #111; color: #FFEEAA }
                                h1 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm red}
                                h2 { font-size:2cm; text-align: center; color: black; text-shadow: 0 0 4mm green}
                         </style></head>
                         <body>
                         <h1>Greetings, Krusty!</h1>
                         <h2>Visitor count: %u</h2>
                         </body></html>\r\n", visitor_count);

                    stream.write(response.as_bytes());
                }
                else {
                    // Requests scheduling
                    let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone()};
                    let (sm_port, sm_chan) = std::comm::stream();
                    sm_chan.send(msg);

                    incoming_ip = ~"128.143.";
                    let mut wahoo_index = 0;                    
                    if checkWahoo(incoming_ip)
                    {
                        println("Found a Charlottesville address");
                        do local_wahoo_index.write |count| {
                            (*count) += 1;
                            wahoo_index = (*count);
                        }
                        println(fmt!("Wahoo requests end at index: %u", wahoo_index));
                    }
                                        
                    do child_add_vec.write |vec| {
                        let msg = sm_port.recv();
                        if wahoo_index > 0 {
                            (*vec).insert(wahoo_index-1, msg); //place at back of Charlottesville queue
                            println("inserted at back of Charlottesville queue");
                        }
                        else {
                            (*vec).push(msg); // enqueue new request.
                            println("add to queue");
                        }
                    }
                    child_chan.send(""); //notify the new arriving request.
                    println(fmt!("get file request: %?", file_path));
                }
            }
            println!("connection terminates")
        }
    }
}

fn checkWahoo(ip: ~str) -> bool {
    let WAHOO_IP = ~["128.143.", "137.54."]; 
    println(fmt!("ip: %s", ip));
       
    for matcher in WAHOO_IP.iter() {
        if ip.contains(*matcher) {
            return true;
        }
    }
    return false;
}