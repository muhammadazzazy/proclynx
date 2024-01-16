// https://github.com/fdehau/tui-rs/blob/master/examples/user_input.rs
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use sysinfo::{ComponentExt, System, SystemExt, CpuExt, DiskExt};
use unicode_width::UnicodeWidthStr;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::str;
use std::process::Command;
use psutil::process::Process;
use sysinfo::NetworkExt;
use pretty_bytes::converter::convert;

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    messages: Vec<String>,
    output: Vec<String>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            output: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut flag: bool = false;
    let mut num: i32 = 0;
    let mut sys = System::new_all();
    let mut arg = String::new();
    let mut parts: Vec<String>;
    let mut history: Vec<String> = vec![];
    loop {
        terminal.draw(|f| ui(f, &app))?;
        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('e') | KeyCode::Char('E') => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Down => {
                        if flag {
                            if !(app.output.is_empty()) && history.len() <= (num-45).try_into().unwrap() {
                                history.push(app.output.remove(1)); 
                            }                            
                        }
                    },
                    KeyCode::Up => {

                        if flag {
                            if !(history.is_empty()) {
                                app.output.insert(1, history.pop().unwrap());
                            }                            
                        }
                    },
                    KeyCode::Enter => {
                        parts = app.input.split_whitespace().map(|s| s.to_string()).collect();
                        app.output.clear();
                        app.messages.push(app.input.drain(..).collect());
                        match parts[0].as_str() {
                            "uname" => {
                                flag = false;
                                app.output.push(format!("{}", sys.kernel_version().unwrap()))
                            },
                            "release" => {
                                flag = false;
                                app.output.push(format!("{}", sys.os_version().unwrap()))
                            },
                            "hostname" => {
                                flag = false;
                                app.output.push(format!("{}", sys.host_name().unwrap()))
                            },
                            "sysinfo" => {
                                flag = false;
                                app.output = get_system_information(&mut sys);
                            },
                            "sensors" => {
                                flag = false;
                                app.output = get_components_information(&mut sys);
                            },
                            "df" => {
                                flag = false;
                                if parts.len() == 2 {
                                    arg = parts[1][1..].to_string();
                                }
                                app.output = get_disks_information(&mut sys, arg.clone());
                            },
                            "hddtemp" => {
                                flag = false;
                                if parts.len() == 2 {
                                    arg = parts[1][1..].to_string();
                                }
                                app.output = get_hddtemp(&mut sys, arg.clone());
                            },
                            "lscpu" => {
                                flag = false;
                                app.output = get_cpu_information(&mut sys);
                            },
                            "gputemp" => {
                                flag = false;
                                if parts.len() == 2 {
                                    arg = parts[1][1..].to_string();
                                }
                                app.output = get_gputemp(&mut sys, arg.clone());
                            },
                            "kill" => {
                                flag = false;
                                if parts.len() == 2 {
                                    if parts[1].parse::<i32>().is_ok() {
                                        let pid = parts[1].parse::<i32>().unwrap();
                                        kill_by_pid(&mut app, pid);
                                    }
                                    else {
                                        kill_by_name(&mut app, parts[1].clone());
                                    }
                                }
                            },
                            "ignite" => {
                                flag = false;
                                if parts.len() == 2 {
                                    Command::new(parts[1].as_str()).output()?;    
                                }
                            },
                            "ptable" => {
                                num = printptable(&mut app);
                                flag = true;
                            },
                            "clear" => {
                                flag = false;
                                app.output.clear();
                            },
                            "help"=> {
                                app.output.push(format!("COMMANDS .\n"));
                                app.output.push(format!("find (pid) --> retrievs the info of process with (pid)"));
                                app.output.push(format!("ignite --> start new process"));
                                app.output.push(format!("ptable --> prints proces table"));
                                app.output.push(format!("desc --> sort process table descendingly"));
                                app.output.push(format!("sysinfo --> retrieves system info"));
                                app.output.push(format!("kill (pid/name)--> kill process with (pid/name)"));
                                app.output.push(format!("uname --> prints the kernel version"));
                                app.output.push(format!("uname --> prints the kernel version"));
                                app.output.push(format!("release --> prints the OS version"));
                                app.output.push(format!("release --> prints the OS version"));
                                app.output.push(format!("hostname --> prints the hostname"));
                                app.output.push(format!("sensors --> prints the labels of various components with their associated temperatures"));
                                app.output.push(format!("df --> prints the disk filesystem information"));
                                app.output.push(format!("hddtemp --> prints the temperature of the internal HDD/SSD"));
                                app.output.push(format!("lscpu --> lists the processor information"));
                                app.output.push(format!("gputemp --> prints the temperature of the GPU"));
                                app.output.push(format!("network --> prints information pertaining to network utilization"));
                                app.output.push(format!("memory --> prints information pertaining to memory utilization"));
                            },
                            "find" => {
                                if parts.len() == 2 {
                                    let pid = parts[1].parse::<i32>().unwrap();
                                    if let Some(process) = findbypid(pid) {
                                        app.output.push(format!("Process with PID {} found!: {:?}", pid, process.name().unwrap()));
                                        let mut p = process;
                                        app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", "PID","%CPU", "%MEM", "COMMAND"));
                                        match p.cmdline() {
                                            Ok(None) => {},
                                            _=> {app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", p.pid(), p.cpu_percent().unwrap(), p.memory_percent().unwrap(), p.cmdline().unwrap().expect("Oops something went wrong!").to_string()));},
                                        }
                                        // app.output.push(format!("Process with PID {} found!: {:?}", pid, process.cpu_percent().unwrap()));
                                    } else {
                                        app.output.push(format!("Process not found with PID {}", pid));
                                    }
                                }
                            },
                            "network" =>{
                                networkuti(&mut app);
                            },
                            "memory" => {
                                memutil(&mut app)
                            },
                            "desc" =>{
                                desc(&mut app);
                            },
                            _ => {app.output.push(format!("command not found"))},
                        }
                        
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Esc => {
                        app.input_mode = InputMode::Normal;
                    }
                    _ => {}
                },
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default().fg(Color::Yellow),
            InputMode::Editing => Style::default().fg(Color::Green),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    let output: Vec<ListItem> = app
        .output
        .iter()
        .enumerate()
        .map(|(_i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}", m)))];
            ListItem::new(content)
        })
        .collect();
    let output =
        List::new(output).block(Block::default().borders(Borders::ALL).title("Output")).style(Style::default().fg(Color::Green));


    
    f.render_widget(output, chunks[2]);
}


fn get_system_information(sys: &System) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    vec.push(format!("Name: {}", sys.name().unwrap()));
    vec.push(format!("Kernel version: {}", sys.kernel_version().unwrap()));
    vec.push(format!("OS version: {}", sys.os_version().unwrap()));
    vec.push(format!("Host name: {}", sys.host_name().unwrap()));
    return vec;
}

fn get_components_information(sys: &mut System) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    for component in sys.components() {
        vec.push(format!("{:?}", component));
    }
    return vec;
}

fn get_hddtemp(sys: &mut System, arg: String) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    match arg.as_str() {
        "" => {
            for component in sys.components_mut() {
                if component.label().contains("SSD") || component.label().contains("HDD"){
                    vec.push(format!("{}: {:?}°C", component.label(), component.temperature()));
                    component.refresh();
                }
            }            
        },
        "max" => {
            for component in sys.components_mut() {
                if component.label().contains("SSD") || component.label().contains("HDD"){
                    vec.push(format!("{}: {:?}°C", component.label(), component.max()));
                    component.refresh();
                }
            }
        },
        "crit" => {
            for component in sys.components_mut() {
                if component.label().contains("SSD") || component.label().contains("HDD"){
                    vec.push(format!("{}: {:?}°C", component.label(), component.critical().unwrap()));
                    component.refresh();
                }
            }
        },
        _ => {},
    }   
    return vec;
}

fn get_disks_information(sys: &mut System, arg: String) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    let base: u64 = 2;
    let mut power: u32 = 0;
    match arg.as_str() {
        "" => {power = 0;},
        "k" => {power = 10;},
        "m" => {power = 20;},
        _ => {},
    }
    vec.push(format!("{:<50} {:<50} {:<50} {:<50} {:<50} {:<50}", "Name", "Mount Point", "Filesystem", "Total Space", "Available Space", "Used Space"));
    for disk in sys.disks() {
        vec.push(format!("{:<50} {:<50} {:<50} {:<50} {:<50} {:<50}", disk.name().to_str().unwrap(), disk.mount_point().to_str().unwrap(), str::from_utf8(disk.file_system()).unwrap(), disk.total_space()/(base.pow(power)), disk.available_space()/(base.pow(power)), disk.total_space()/(base.pow(power)) - disk.available_space()/(base.pow(power))));
    }
    return vec;
}

fn get_cpu_information(sys: &mut System) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    vec.push(format!("{:<50} {:<50} {:<50} {:<50}", "Brand", "Vendor ID", "Name", "Frequency"));
    for cpu in sys.cpus() {
        vec.push(format!("{:<50} {:<50} {:<50} {:<50}", cpu.brand(), cpu.vendor_id(), cpu.name(), cpu.frequency()));
    }
    return vec;
}

fn get_gputemp(sys: &mut System, arg: String) -> Vec<String> {
    let mut vec: Vec<String> = vec![];
    match arg.as_str() {
        "" =>  {
            for component in sys.components_mut() {
                if component.label().contains("gpu") {
                    vec.push(format!("{}: {}°C", component.label(), component.temperature()));
                    component.refresh();
                }
            }       
        },
        "max" => {
            for component in sys.components_mut() {
                if component.label().contains("gpu"){
                    vec.push(format!("{}: {}°C", component.label(), component.max()));
                    component.refresh();
                }
            }   
        },
        _ => {}
    }
    return vec;
}

fn printptable(app: &mut App) -> i32 {
    let mut num: i32 = 0;
    let processes = psutil::process::processes().unwrap();
    app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", "PID", "%CPU", "%MEM", "COMMAND"));
    for process in processes {
        let mut p = process.unwrap();
        match p.cmdline() {
            Ok(None) => {},
            _=> {
                num = num + 1;
                app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", p.pid(), p.cpu_percent().unwrap(), p.memory_percent().unwrap(), p.name().unwrap()));
            },
        }
    }
    return num;
}

fn kill_by_pid(app: &mut App, pid: i32) {
    match kill(Pid::from_raw(pid), Signal::SIGTERM) {
        Ok(_) => app.output.push(format!("Process with killed successfully.\n")),
        Err(e) => app.output.push(format!("Error killing process: {}\n", e)),
    }
}

fn kill_by_name(app: &mut App, name: String) {
    let processes = psutil::process::processes().unwrap();
    for process in processes {
        let mut p = process.unwrap();
        match p.cmdline() {
            Ok(None) => {},
            _=> {
                if name == p.name().unwrap().to_string() {
                    match kill(Pid::from_raw(p.pid().try_into().unwrap()), Signal::SIGTERM) {
                        Ok(_) => app.output.push(format!("Process with killed successfully.\n")),
                        Err(e) => app.output.push(format!("Error killing process: {}\n", e)),
                    }
                }

            },
        }
    }
}

pub fn findbypid(pid: i32) -> Option<Process> {
    match Process::new(pid.try_into().unwrap()) {
        Ok(process) => Some(process),
        Err(_) => None
    }
}


fn networkuti(app: &mut App) {
    let mut system = System::new_all();
    system.refresh_all();

    for (interface_name, network_interface) in system.networks() {
        app.output.push(format!("Interface {}: transmitted: {}, received: {}", interface_name, network_interface.total_packets_transmitted(), network_interface.total_packets_received()));
    }
}

fn memutil(app: &mut App) {
    let s = System::new_all();
    app.output.push(format!("Total Memory: {}", convert(s.total_memory()as f64)));
    app.output.push(format!("Used Memory: {}", convert(s.used_memory()as f64)));
    app.output.push(format!("Free Memory: {}", convert(s.free_memory()as f64)));

}

fn desc(app: &mut App) {
    let mut processes = psutil::process::processes().unwrap();
    processes.reverse();
    app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", "PID","%CPU", "%MEM", "COMMAND"));
    app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", "PID", "%CPU", "%MEM", "COMMAND"));
    for process in processes {
        let mut p = process.unwrap();
        match p.cmdline() {
            Ok(None) => {},
            _=> {
                app.output.push(format!("{:<30} {:<30} {:<30} {:<30}", p.pid(), p.cpu_percent().unwrap(), p.memory_percent().unwrap(), p.name().unwrap()));
            },
        }
    }
}

// pub fn pstree_new(sys: &mut System) {
//     let processes = SystemExt::processes(sys);
//     let mut sorted_keys: Vec<_> = processes.keys().collect();
//     sorted_keys.sort();
//     let mut process_map: HashMap<i32, Vec<i32>> = HashMap::new();
//     let  mut tree = ptree::TreeBuilder::new("root".to_string());
//     let mut muttree = &mut tree;
//     let mut resulttree: StringItem = muttree.build();

//     for pid in sorted_keys {
//         // let new = ptree::TreeBuilder::new("root".to_string()).begin_child(processes[pid].name().to_string());
//         // let neww = tree.begin_child(processes[pid].name().to_string()).build();
//         let process = &processes[pid];
//         match Process::parent(process) {
//             Some(parent_pid) => {
//                 process_map
//                     .entry(Pid::as_u32(parent_pid) as i32)
//                     .or_insert_with(Vec::new)
//                     .push(Pid::as_u32(*pid) as i32);
//             }
//             None => {
//                 // If there is no parent process, assume it is the root process
//                 process_map.entry(0).or_insert_with(Vec::new).push(Pid::as_u32(*pid) as i32);
//             }
//         }
//         //let results = ptree::print_tree(&neww);
//     }

//     let new_keys: Vec<_> = process_map.keys().collect();
//     for pid in new_keys{
//         if process_map[pid].len() >= 1 {
//             let newleaf = muttree.add_empty_child(process_map[pid][0].to_string());
//             muttree = newleaf.add_empty_child(" ".to_string());
//             resulttree = muttree.build();
//         }

//         else{
//             let newbranch = muttree.begin_child(process_map[pid][0].to_string());
            
//             for i in 1..process_map[pid].len(){
//                 let newleaf = newbranch.add_empty_child(process_map[pid][i].to_string());
//                 resulttree = newleaf.build();
//             }
//         }
//     }

//     let results = ptree::print_tree(&resulttree);

    

// }
