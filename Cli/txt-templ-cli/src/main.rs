use std::{fs::File, path::PathBuf, io::{self, Write, Read}, process::Command, env};
use clap::Parser;
use txt_templ_compiler::template::Template;
use txt_templ_compiler::{UserContent, UserContentState};
use once_cell::sync::Lazy;
use anyhow::Context;
use giveup::Giveup;


// The default path to the file which contains the configuration
// for the UserContentState
const USER_CONTENT_STATE_DEFAULT: &str = ".template_content_state.yaml";
// The environment variable containing the path to another
// file which contains the configuration for the UserContentState
// If this environment variable is set, its path will overwrite the default one
const USER_CONTENT_STATE_FILE_ENV: &str = "TEMPLATE_CONTENT_STATE_FILE";
// Name of the environment variable which contains the user's default editor name
const EDITOR: &str = "EDITOR";
// Name of a default editor assumed to be installed on most systems and
// usable for most users
const EDITOR_DEFAULT: &str = "nano";
// Name of the temporary file used for editing the user content
const TEMP_FILE_NAME: &str = "content.yaml";


#[derive(Parser, Debug)]
#[command(about = "Fill out templates")]
struct Args {
    /// List of paths to template files
    #[arg(long = "template", short, value_name = "FILE")]
    template_file: PathBuf,
    /// Path to content state file
    #[arg(long = "content-state", short, value_name = "FILE")]
    content_state_file: Option<PathBuf>,
    /// Path to content file
    #[arg(long = "content", short = 'C', value_name = "FILE")]
    content_file: Option<PathBuf>,
    /// Write the content draft to stdout. This will not compile the template
    /// and will irgnore the `content_file` flag.
    #[arg(long, short)]
    draft: bool,
}


struct WithUserContentDraft(UserContent);
struct WithUserContent(UserContent);

trait InputState {}
impl InputState for WithUserContentDraft {}
impl InputState for WithUserContent {}

// Struct assembling all user inputs to compile a template
struct Inputs<S: InputState> {
    // Inputs which are expected to be already persent before the program is stared
    template: Template,
    ucs: UserContentState,
    // State to optionally store the user content after it
    // was entered during runtime
    uc: S,
}

// Operations performed before getting the user content
impl Inputs<WithUserContentDraft> {
    fn new(template_file: &PathBuf, ucs_file: &Option<PathBuf>) -> anyhow::Result<Self> {
        let template = Self::get_template(template_file)?;
        let ucs = Self::get_user_content_state(ucs_file)?;

        // Calculate the user content draft
        let uc_draft = template.required().draft_user_content();
        let uc_draft = WithUserContentDraft(uc_draft);

        Ok(Self{ template, ucs, uc: uc_draft })
    }

    // Read and parse the given template file
    fn get_template(template_file: &PathBuf) -> anyhow::Result<Template> {    
        // Read the template
        let mut file = File::open(template_file)
            .context("Failed to open the template source file")?;
        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .context("Failed to read the template source file")?;

        log::trace!("Successfully read content of template file:\n{}", &buf);

        let template = Template::parse(&buf).context("Parse error")?;
        log::trace!("Successfully parsed content of template file into a valid template:\n{:?}", &template);
        Ok(template)
    }

    // Read and parse the UserContentState from a file.
    // This file used is either the default at USER_CONTENT_STATE_DEFAULT
    // or the file set in USER_CONTENT_STATE_FILE_ENV
    fn get_user_content_state(ucs_file: &Option<PathBuf>) -> anyhow::Result<UserContentState> {
        // Read the UserContentState file
        let mut file = if let Some(path) = ucs_file {
            File::open(&path)
                .with_context(|| format!("Failed to open passed content state file {}", path.display()))?
        } else {
            // User environment or default because the user did not specify a path
            match env::var(USER_CONTENT_STATE_FILE_ENV) {
                Ok(file_name) => {
                    File::open(&file_name)
                        .with_context(|| format!("Failed to open file {} containing the user content state", &file_name))?
                },
                Err(_) => {
                    let path = dirs::home_dir().context("Failed to get $HOME directory")?
                        .join(USER_CONTENT_STATE_DEFAULT);
                    File::open(&path)
                        .with_context(|| format!("Failed to open default file {} containing the user content state",
                            USER_CONTENT_STATE_DEFAULT))?
                },
            }
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf)
            .context("Failed to read the file contaiing the user content state")?;

        let ucs = serde_yaml::from_str(&buf)?;
        log::trace!("Successfully read and parsed user content state:\n{:?}", &ucs);    
        Ok(ucs)
    }

    // Get the keys section of the draft  as a YAML string
    fn keys_yaml(&self) -> anyhow::Result<String> {
        let yaml = serde_yaml::to_string(&self.uc.0.keys)
            .context("Failed to convert keys section of draft to YAML")?;
        Ok(yaml)
    }

    // Get the choices section of the draft as a YAML string
    fn choices_yaml(&self) -> anyhow::Result<String> {
        let yaml = serde_yaml::to_string(&self.uc.0.choices)
            .context("Failed to convert choices section of draft to YAML")?;
        Ok(yaml)
    }

    // Put additional context and available options into the temporary file which
    // will be used for the user to enter their content
    fn prepare_user_content_file<F: Write>(&self, mut file: F) -> anyhow::Result<()> {
        let indent  = || -> String { " ".repeat(2) };

        let mut draft_buf = String::new();
        draft_buf.push_str("keys:\n");  // Begin YAML `keys` section
        draft_buf.push_str(&format!("{}# <key>: <content>\n", indent()));
        // Add all key entries from the draft as YAML
        self.keys_yaml()?.lines().for_each(|line| {
            draft_buf.push_str(&format!("{}{}\n", indent(), line));
        });

        draft_buf.push_str("choices:\n");  // Begin YAML `choices` section
        draft_buf.push_str(&format!("{}# <option>: <choice>\n", indent()));
        // Add all choice entries from the draft as YAML.
        // The content field will either be empty or the default content
        draft_buf.push_str(&format!("{}# Default literals:\n", indent()));
        self.choices_yaml()?.lines().for_each(|line| {
            draft_buf.push_str(&format!("{}{}\n", indent(), line));
        });

        draft_buf.push_str(&format!("\n{}# All available choices \
            (For each option the last choice not commented out will be used):\n", indent()));
        // Write all choices for all options to the file as YAML comments
        // so the user can quickly uncomment the option they choose
        const MAX_PREVIEW_LEN: usize = 31;  // Maximum length of content preview
        for (option, choices) in self.ucs.options.iter() {
            // Check the option is found in the tempalte before adding it
            if self.uc.0.choices.get(option).is_some() {
                let mut max_len = usize::MIN;
                for choice in choices.keys() {
                    if option.len() + choice.len() > max_len {
                        max_len = option.len() + choice.len();
                    }
                }
                for (choice, content) in choices.iter() {
                    draft_buf.push_str(&format!("{}# {}: {}", indent(), option, choice));
                    // Append a comment containing the content associated with the current choice
                    let space = max_len - (option.len() + choice.len()) + 4;  // Make all comments start on the same column
                    match content.len() {
                        0..=MAX_PREVIEW_LEN => draft_buf.push_str(&format!("{}# -> \"{}\"\n", " ".repeat(space), content)),
                        _ => draft_buf.push_str(&format!("{}# -> \"{}\"\n", " ".repeat(space), &content[..MAX_PREVIEW_LEN-3])),
                    }
                }
            } 
        }
        
        file.write_all(draft_buf.as_bytes()).context("Failed to write to temporary file")
    }

    // Open a temporary YAML file containg all entries to UserContent
    // in the user's default editor to allow the user to set the values.
    // The create an instance of UserState from the temporary file.
    // Change the state of `self` to `WithUserContent`
    // If the user passed a content file just read this file
    fn get_user_content(self, content_file: &Option<PathBuf>) -> anyhow::Result<Inputs<WithUserContent>> {
        let mut buf = String::new();  // Buffer for user content YAML
        match content_file {
            Some(path) => {  // Read the user content from the file as YAML
                File::open(&path)
                    .with_context(|| format!("Failed to open passed content file {}", path.display()))?
                    .read_to_string(&mut buf).context("Failed to read all contents of passed file")?;
            },
            None => {  // Create a temporary file for the user to enter their content
                // Create a temporary file
                let temp_file_path = {
                    let mut path = env::temp_dir();
                    path.push(TEMP_FILE_NAME);
                    path
                };
                let file = File::create(&temp_file_path)
                    .context("Failed to create temporary file")?;

                // Write the user content draft to the file as YAML
                self.prepare_user_content_file(file).context("Failed to prepare file")?;

                // Open the temp file in the user's preferred editor
                let editor = env::var(EDITOR).unwrap_or_else(|_| EDITOR_DEFAULT.to_owned());
                let exit_status = Command::new(&editor)
                    .arg(&temp_file_path)
                    .status()
                    .with_context(|| format!("Something went wrong opening the temporary file in {}", &editor))?;
                if !exit_status.success() {
                    anyhow::bail!("Editing the user content returned an error")
                }

                // Read the content of the file the user edited
                File::open(&temp_file_path).context("Failed to reopen temporary file")?
                    .read_to_string(&mut buf).context("Failed to read all contents of temporary file")?;
            }
        }

        let uc: UserContent = serde_yaml::from_str(&buf)?;
        log::trace!("Successfully read and parsed user content:\n{:?}", &uc);

        Ok(Inputs {
            template: self.template,
            ucs: self.ucs,
            uc: WithUserContent(uc),
        })
    }
}

// Operations performed after getting the user content
impl Inputs<WithUserContent> {
    // Allow compiling the template after all inputs where assembled successfully
    fn compile(self) -> anyhow::Result<String> {
        let result = self.template.fill_out(self.uc.0, self.ucs)?;
        log::trace!("Successfully filled out tempalte:\n{}", &result);        
        Ok(result)
    }
}

// TODO: Write a usage guide of the CLI

fn main() {
    Lazy::force(&Lazy::new(|| env_logger::init()));

    let args = Args::parse();
    log::trace!("Successfully parsed arguments from command line: {:?}", &args);

    let inputs = Inputs::new(&args.template_file, &args.content_state_file)
        .giveup("Failed to get static content");

    if args.draft {
        // Only create the user content draft and write it to stdout.
        let stdout = io::stdout();
        let handle = stdout.lock();
        inputs.prepare_user_content_file(handle).giveup("Failed to draft content file");
    } else {
        // Compile the template as usual
        let result = inputs.get_user_content(&args.content_file)
            .giveup("Failed to get new content")
            .compile().giveup("Failed to compile template");

        print!("{}", result);
    }
}
