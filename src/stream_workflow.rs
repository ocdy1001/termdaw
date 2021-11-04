use crate::state::*;

use term_basics_linux::*;
use skim::prelude::*;

use std::thread;
use std::sync::{ mpsc };
use std::time::{ Duration, Instant };
use std::io::{ Cursor };

use std::io::{ self };
use ::floww::*;
use std::collections::HashMap;

pub fn run_stream_workflow(proj_sr: usize, buffer_len: usize, state: State, device: sdl2::audio::AudioQueue<f32>){
    let (transmit_to_main, receive_in_main) = mpsc::channel();

    launch_stream_thread(transmit_to_main);
    stream_partner(state, device, proj_sr, buffer_len, receive_in_main);
}

#[derive(PartialEq)]
enum StreamThreadMsg{
    None, Quit, Stop, Play, Feed(Vec<Vec<Point>>),
}

fn launch_stream_thread(transmit_to_main: mpsc::Sender<StreamThreadMsg>){
    thread::spawn(move || {
        let mut tracks = vec![vec![]; 4];
        let map: HashMap<String, usize> = [
            ("ride".to_string(), 0),
            ("hihat".to_string(), 1),
            ("kick".to_string(), 2),
            ("snare".to_string(), 3)
        ].iter().cloned().collect();

        loop{
            if let Ok(res) = io::stdin().lock().decoded(){
                let msgs = unpacket(&mut tracks, &map, res);
                println!("MSGS: {:?}", msgs);
                println!("TRACKS: {:?}", tracks);
            } else {
                println!("OOF AUW RIP");
            }
        }
    });
}

fn stream_partner(mut state: State, device: sdl2::audio::AudioQueue<f32>, proj_sr: usize, buffer_len: usize,
                receive_in_main: mpsc::Receiver<StreamThreadMsg>){
    let mut playing = false;
    let mut since = Instant::now();
    let mut millis_generated = 0f32;
    loop {
        if let Ok(rec) = receive_in_main.try_recv(){
            macro_rules! check_loaded{
                ($b:block) => {
                    if !state.loaded{
                        println!("{}State not loaded!", UC::Red);
                    } else {
                        $b;
                    }
                }
            }
            match rec{
                StreamThreadMsg::Quit => {
                    break;
                },
                StreamThreadMsg::Feed(data) => {
                    check_loaded!({
                        // device.clear();
                        // device.pause();
                        // playing = false;
                        // state.render();
                    });
                },
                StreamThreadMsg::Play => {
                    check_loaded!({
                        playing = true;
                        since = Instant::now();
                        millis_generated = 0.0;
                        device.resume();
                    });
                },
                StreamThreadMsg::Stop => {
                    playing = false;
                    device.pause();
                },
                _ => {}
            }
        }
        if playing{
            if !state.loaded {
                playing = false;
            } else {
                let time_since = since.elapsed().as_millis() as f32;
                // render half second in advance to be played
                while time_since > millis_generated - 0.5 {
                    let chunk = state.g.render(&state.sb, &mut state.fb, &mut state.host);
                    let chunk = chunk.unwrap();
                    let stream_data = chunk.clone().interleave();
                    device.queue(&stream_data);
                    millis_generated += buffer_len as f32 / proj_sr as f32 * 1000.0;
                    state.fb.set_time_to_next_block();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
