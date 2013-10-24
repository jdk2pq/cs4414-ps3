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
use std::{os, str, io, run};
use extra::arc;
use std::comm::*;
use gash::handle_cmdline;

mod gash;

static PORT:    int = 4414;
static IP: &'static str = "127.0.0.1";

struct sched_msg {
    stream: Option<std::rt::io::net::tcp::TcpStream>,
    filepath: ~std::path::PosixPath
}

//allows for server-side gashing replacement of '<--!#exec cmd="{gash-command}" -->'
//admittedly lousy implementation, and only replaces the first instance of a command at the moment
//but i've been working on it for hours, and it does something, and i'm happy
fn exec_command(file_data: &str) -> ~str {
    let mut build_result: ~[~str] = ~[~""];
    // while (file_data.contains("<--!#exec cmd=")) { //hopefully i can get this working to replace every instance of a command
        println("replacing SSI gashing!");
        let index = file_data.find_str("<--!#exec cmd=").unwrap();
        build_result.push(file_data.slice_to(index).to_owned());
        let next_part = file_data.slice_from(index);
        let command_stmt = next_part.slice_to(next_part.find_str("\" -->").unwrap()+5);
        let mut cmd = command_stmt.replace("<--!#exec cmd=\"", "");
        cmd = cmd.replace("\" -->", "");
        // println(fmt!("command is %?", cmd)); //for testing
        let x: Option<run::ProcessOutput> = handle_cmdline(cmd);
        // println(fmt!("test %?", str::from_utf8(handle_cmdline("date").unwrap().output))); //for testing 
        build_result.push(str::from_utf8(x.unwrap().output));
        build_result.push(next_part.slice_from(next_part.find_str("-->").unwrap()+3).to_owned());
    // }
    return build_result.concat();
}

fn main() {
    //Safe visitor count
    let visitor_count: uint = 0;
    let shared_vis_count = arc::RWArc::new(visitor_count);

    //Normal scheduler queue
    let req_vec: ~[sched_msg] = ~[];
    let shared_req_vec = arc::RWArc::new(req_vec);
    let add_vec = shared_req_vec.clone();
    let take_vec = shared_req_vec.clone();
    let (port, chan) = stream();
    let chan = SharedChan::new(chan);

    //WahooFirst scheduler queue (hence the "_char" suffix for "Charlottesville")
    let req_vec_char: ~[sched_msg] = ~[];
    let shared_req_vec_char = arc::RWArc::new(req_vec_char);
    let add_vec_char = shared_req_vec_char.clone();
    let take_vec_char = shared_req_vec_char.clone();
    let (port_char, chan_char) = stream();
    let chan_char = SharedChan::new(chan_char);
    
    // add file requests into queue.
    do spawn {
        let (sm_port, sm_chan) = stream();
        let (sm_port_char, sm_chan_char) = stream();

        do spawn {
            loop {
                //Serve WahooFirst...first
                while (sm_port_char.peek()) {
                    let mut tf: sched_msg = sm_port_char.recv(); // wait for the dequeued request to handle
                    match io::read_whole_file_str(tf.filepath) { // killed if file size is larger than memory size.
                        Ok(file_data) => {
                            println(fmt!("begin serving file from wa-queue [%?]", tf.filepath));
                            println(fmt!("file size is %?", tf.filepath.get_size().unwrap()));
                            if (file_data.contains("<--!#exec cmd=")) {
                                let file_data_str = exec_command(file_data);
                                // A web server should always reply a HTTP header for any legal HTTP request.
                                tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                                println(fmt!("file contains: %?", file_data_str));
                                tf.stream.write(file_data_str.as_bytes());
                                println(fmt!("finish file [%?]", tf.filepath));
                            } else {
                                // A web server should always reply a HTTP header for any legal HTTP request.
                                tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                                println(fmt!("file contains: %?", file_data));
                                tf.stream.write(file_data.as_bytes());
                                println(fmt!("finish file [%?]", tf.filepath));
                            }
                        }
                        Err(err) => {
                            println(err);
                        }
                    }
                }
                let mut tf: sched_msg = sm_port.recv(); // wait for the dequeued request to handle
                match io::read_whole_file_str(tf.filepath) { // killed if file size is larger than memory size.
                    Ok(file_data) => {
                        println(fmt!("begin serving file [%?]", tf.filepath));
                        println(fmt!("file size is %?", tf.filepath.get_size().unwrap()));
                        if (file_data.contains("<--!#exec cmd=")) {
                            let file_data_str = exec_command(file_data);
                            // A web server should always reply a HTTP header for any legal HTTP request.
                            tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                            println(fmt!("file contains: %?", file_data_str));
                            tf.stream.write(file_data_str.as_bytes());
                            println(fmt!("finish file [%?]", tf.filepath));
                        } else {
                            // A web server should always reply a HTTP header for any legal HTTP request.
                            tf.stream.write("HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream; charset=UTF-8\r\n\r\n".as_bytes());
                            println(fmt!("file contains: %?", file_data));
                            tf.stream.write(file_data.as_bytes());
                            println(fmt!("finish file [%?]", tf.filepath));
                        }
                    }
                    Err(err) => {
                        println(err);
                    }
                }
            }
        }

        loop {
            //Serve WahooFirst...first
            while (port_char.peek()) {
                port_char.recv(); // wait for arrving notification
                do take_vec_char.write |vec| {
                    if ((*vec).len() > 0) {
                        //Find smallest file in browser and serve that; SRTP 
                        let mut min = (*vec)[0].filepath.get_size();
                        let mut index = 0;
                        let mut i = 1;
                        let times = (*vec).len();
                        do times.times {
                            if (i < (*vec).len()) {
                                if (min > (*vec)[i].filepath.get_size()) {
                                    min = (*vec)[i].filepath.get_size();
                                    index = i;
                                    i = i+1;
                                }
                            }
                        }
                        // LIFO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                        let tf = (*vec).remove(index);
                        println(fmt!("shift from wa-queue, size: %ud", (*vec).len()));
                        sm_chan_char.send(tf); // send the request to send-response-task to serve.
                    }
                }
            } 

            port.recv(); // wait for arrving notification
            do take_vec.write |vec| {
                if ((*vec).len() > 0) {
                    //Find smallest file in browser and serve that; SRTP 
                    let mut min = (*vec)[0].filepath.get_size();
                    let mut index = 0;
                    let mut i = 1;
                    let times = (*vec).len();
                    do times.times {
                        if (i < (*vec).len()) {
                            if (min > (*vec)[i].filepath.get_size()) {
                                min = (*vec)[i].filepath.get_size();
                                index = i;
                                i = i+1;
                            }
                        }
                    }
                    // LIFO didn't make sense in service scheduling, so we modify it as FIFO by using shift_opt() rather than pop().
                    let tf = (*vec).remove(index);
                    // let tf = tf_opt.unwrap();
                    println(fmt!("shift from queue, size: %ud", (*vec).len()));
                    sm_chan.send(tf); // send the request to send-response-task to serve.
                }
            }
        }
    }

    let ip = match FromStr::from_str(IP) { 
        Some(ip) => ip, 
        None => { 
            println(fmt!("Error: Invalid IP address <%s>", IP));
            return;
        },
    };

    let socket = net::tcp::TcpListener::bind(SocketAddr {ip: ip, port: PORT as u16});

    println(fmt!("Listening on %s:%d ...", ip.to_str(), PORT));
    let mut acceptor = socket.listen().unwrap();

    // we can limit the incoming connection count.
    // for stream in acceptor.incoming().take(10 as uint) {
        for stream in acceptor.incoming() {
            let mut stream = stream;
            let mut peer_name: ~str = ~"";
            match stream {
                Some(ref mut s) => {
                    match s.peer_name() {
                        Some(peername) => {
                        //println( fmt!("Peer address: %s \n", peername.to_str() ));
                        peer_name = peername.to_str();
                    }
                    None => ()
                }
            }
            None => ()
        }
        let stream = Cell::new(stream);
        let peer_name = Cell::new(peer_name);
        let local_vis_count = shared_vis_count.clone();
        // Start a new task to handle the each connection
        let child_chan = chan.clone();
        let child_add_vec = add_vec.clone();
        let child_chan_char = chan_char.clone();
        let child_add_vec_char = add_vec_char.clone();
        do spawn {
            let mut vis_count = 0;
            let peer_name = peer_name.take();
            do local_vis_count.write |count| {
                (*count) = (*count)+1;
                vis_count = (*count);
            }
            let mut stream = stream.take();
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
                        </body></html>\r\n", vis_count);

                    stream.write(response.as_bytes());
                }
                else {
                    // Requests scheduling
                    if (peer_name.starts_with("128.143.") || peer_name.starts_with("137.54.")) {
                        println(fmt!("peer_name fits into Charlottesville. Going into Wa-queue"));
                        let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone()};
                        let (sm_port_char, sm_chan_char) = std::comm::stream();
                        sm_chan_char.send(msg);

                        do child_add_vec_char.write |vec| {
                            let msg = sm_port_char.recv();
                            (*vec).push(msg); // enqueue new request.
                            println(fmt!("add to wa-queue, size: %ud", (*vec).len()));
                        }
                        child_chan_char.send(""); //notify the new arriving request.
                        println(fmt!("get file request: %?", file_path));
                    } else {
                        let msg: sched_msg = sched_msg{stream: stream, filepath: file_path.clone()};
                        let (sm_port, sm_chan) = std::comm::stream();
                        sm_chan.send(msg);

                        do child_add_vec.write |vec| {
                            let msg = sm_port.recv();
                            (*vec).push(msg); // enqueue new request.
                            println(fmt!("add to queue, size: %ud", (*vec).len()));
                        }
                        child_chan.send(""); //notify the new arriving request.
                        println(fmt!("get file request: %?", file_path));
                    }
                }
            }
            println!("connection terminates")
        }
    }
}
