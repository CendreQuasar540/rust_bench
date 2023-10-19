use std::{fs::File, io::Write, env};

/// Project to shared source code between all binary project refered in the
/// worskpace

pub const DEFAULT_LOOP_CNT: usize = 1000;
pub const LAZY_START : u64 = 2000;

/// Just a enum to tag task "master" and "slave"
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskId {
    A = 0, // Master task
    B,
    C,
    D,
}

impl TaskId {
    pub fn nameof(&self) -> &'static str {
        return match self {
            TaskId::A => "A",
            TaskId::B => "B",
            TaskId::C => "C",
            TaskId::D => "D",
        };
    }
}

/// Basic structure to save app settings
/// It recomended to use it through a singleton pattern to keep
/// parameters synchronise in all part of the app.
#[derive(Clone, PartialEq, Eq)]
pub struct AppSettings {
    pub limit : usize,
    pub display_iter : bool,
    pub draw_chart : bool,
    pub worskpace : String,
    pub output_file : String,
}

/// Settings class implementation
impl AppSettings {
    pub fn default() -> Self {
        return Self {
            limit : DEFAULT_LOOP_CNT,
            display_iter : false,
            draw_chart : true,
            worskpace : String::default(),
            output_file : String::default(),
        };
    }
}

/// Format results generate by the benchmark and print it a file with the given name in parameter
/// Header:
pub fn format_str(col_a : &Vec<(TaskId, u128)>,  col_b : &Vec<(TaskId, u128)>, title : &str) -> std::io::Result<()>
{
    if col_a.len() == col_b.len() && !col_a.is_empty()
    {
        let name_a = col_a.first().unwrap().0.nameof();
        let name_b = col_b.first().unwrap().0.nameof();
        let mut str_diff = String::from(format!("task{name_a};task{name_b};difference\n"));
        let mut file_diff: File = File::create(title)?;
        for i in 0..col_a.len() {
            str_diff += (col_a[i].1.to_string() + ";").as_ref();
            str_diff += (col_b[i].1.to_string() + ";").as_ref();
            str_diff += ((col_a[i].1.abs_diff(col_b[i].1)).to_string() + ";\n").as_str();
        }
        file_diff.write_all(str_diff.as_bytes())?;
    }
    Ok(())
}

///Read arguments send to app at startup and return a structure with initialized flags and counter
pub fn read_app_argument() -> AppSettings {
    //Init var and get args
    let mut settings = AppSettings::default();
    let vargs : Vec<String> = env::args().into_iter().collect();

    let helpmsg: &str =
"Rust runtime asynchroneous and threading benchmark library (=>just another useless project). Option supported:
    -h(elp): Display this message
    -d(ebug): Display loop iteration in the console
    -l(imit) <value>: Set new iteration limit for the benchmark (default: 1000)
    -o(output) <path>";

    //Match args to an option
    let mut jump_arg : bool = false;
    settings.worskpace = vargs[0].clone();
    for i in 1..vargs.len()
    {
        if jump_arg {
            // Just Ignrore this case and check next argument
            jump_arg = false;
        }
        else if (vargs[i] == "-l" || vargs[i] == "-limit") && i + 1 < vargs.len() {
            settings.limit = vargs[i + 1].parse::<usize>().unwrap_or(DEFAULT_LOOP_CNT);
            jump_arg = true;
        }
        else if (vargs[i] == "-o" || vargs[i] == "-output") && i + 1 < vargs.len() {
            settings.output_file = vargs[i + 1].clone();
            jump_arg = true;
        }
        else if vargs[i] == "-d" || vargs[i] == "-debug" {
            settings.display_iter = true; }
        else if vargs[i] == "-h" || vargs[i] == "-help" {
            println!("{helpmsg}"); }
        else {
            println!("Aguments unknown: {}, check help (-h) for more detail about supported argument", vargs[i]);
        }
    }
    return  settings;
}