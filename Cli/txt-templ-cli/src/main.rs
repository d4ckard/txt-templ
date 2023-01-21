use std::{fs::File, path::PathBuf, io::{prelude::*, Read}, process::Command, env};
use clap::Parser;
use txt_templ_compiler::template::Template;
use txt_templ_compiler::{UserContent, UserContentState};
use once_cell::sync::Lazy;
use anyhow::Context;
use giveup::Giveup;


// The default path to the file which contains the configuration
// for the UserContentState
const USER_CONTENT_STATE_DEFAULT: &str = "$HOME/.template_content_state.yaml";
// The environment variable containing the path to another
// file which contains the configuration for the UserContentState
// If this environment variable is set, its path will overwrite the default one
const USER_CONTENT_STATE_FILE_ENV: &str = "TEMPLATE_STATE_FILE";
// Name of the environment variable which contains the user's default editor name
const EDITOR: &str = "EDITOR";
// Name of a default editor assumed to be installed on most systems and
// usable for most users
const EDITOR_DEFAULT: &str = "nano";


#[derive(Parser, Debug)]
#[command(about = "Fill out templates")]
struct Args {
    /// Set the template source file
    #[arg(value_name = "FILE")]
    file: PathBuf,
}

trait TemplateYamlExt {
    type Error;
    fn user_content_yaml(&self) -> Result<String, Self::Error>;
}

impl TemplateYamlExt for Template {
    type Error = anyhow::Error;

    // TODO: Improve option choice usability
    // maybe by listing all option being commented out
    fn user_content_yaml(&self) -> Result<String, Self::Error> {
        let uc_draft = self.required().draft_user_content();
        let yaml = serde_yaml::to_string(&uc_draft)
            .context("Failed to convert user content draft to YAML")?;
        Ok(yaml)
    }
}

// Read and parse the given template file
fn get_template(file_name: &PathBuf) -> anyhow::Result<Template> {    
    // Read the template
    let mut file = File::open(file_name)
        .context("Failed to open the template source file")?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .context("Failed to read the template source file")?;

    log::trace!("Successfully read content of template file:\n{}", &buf);

    // Parse the template
    let template = Template::parse(&buf).context("Parse error")?;
    Ok(template)
}

// Read and parse the UserContentState from a file.
// This file used is either the default at USER_CONTENT_STATE_DEFAULT
// or the file set in USER_CONTENT_STATE_FILE_ENV
fn get_user_content_state() -> anyhow::Result<UserContentState> {
    // Read the UserContentState file
    let mut file = match env::var(USER_CONTENT_STATE_FILE_ENV) {
        Ok(file_name) => {
            File::open(&file_name)
                .with_context(|| format!("Failed to open file {} containing the user content state", &file_name))?
        },
        Err(_) => {
            File::open(USER_CONTENT_STATE_DEFAULT)
                .with_context(|| format!("Failed to open default file {} containing the user content state",
                    USER_CONTENT_STATE_DEFAULT))?
        },
    };

    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .context("Failed to read the file contaiing the user content state")?;

    let ucs: UserContentState = serde_yaml::from_str(&buf)?;
    Ok(ucs)
}

// Open a temporary YAML file containg all entries to UserContent
// in the user's default editor to allow the user to set the values.
// The create an instance of UserState from the temporary file.
fn get_user_content(template: &Template) -> anyhow::Result<UserContent> {
    // Create a temporary file
    let temp_file_path = {
        let mut path = env::temp_dir();
        path.push("user-content.yaml");
        path
    };
    let mut file = File::create(&temp_file_path).context("Failed to create temporary file")?;

    // Write the user content draft to the file as YAML
    let yaml = template.user_content_yaml()?;
    file.write_all(yaml.as_bytes())
        .context("Failed to write YAML to temporary file")?;

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
    let mut buf = String::new();
    File::open(&temp_file_path).context("Failed to reopen temporary file")?
        .read_to_string(&mut buf).context("Failed to read all contents of temporary file")?;

    let uc: UserContent = serde_yaml::from_str(&buf)?;
    Ok(uc)
}


// TODO: Improve draft-editing usablility
// TODO: Write a usage guide of the CLI

fn main() {
    Lazy::force(&Lazy::new(|| env_logger::init()));

    let args = Args::parse();
    log::trace!("Successfully parsed arguments from command line: {:?}", &args);

    let template = get_template(&args.file).giveup("Template Error");
    log::trace!("Successfully parsed content of template file into a valid template:\n{:?}", &template);

    let ucs = get_user_content_state().giveup("Content State Error");
    log::trace!("Successfully read and parsed user content state:\n{:?}", &ucs);    

    let uc = get_user_content(&template).giveup("Content Error");
    log::trace!("Successfully read and parsed user content:\n{:?}", &uc);

    // Fill out the template
    let result = template.fill_out(uc, ucs).giveup("Fill-Out Error");
    log::trace!("Successfully filled out tempalte:\n{}", &result);

    print!("{}", result);
}
