use std::path::Path;

use crate::{
    code::{language::Language, refactor},
    git,
};

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<git::Error> for Error {
    fn from(error: git::Error) -> Self {
        Error {
            message: error.to_string(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error {
            message: error.to_string(),
        }
    }
}

impl From<refactor::Error> for Error {
    fn from(error: refactor::Error) -> Self {
        Error {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error {
            message: error.to_string(),
        }
    }
}

fn update_mod_config(path: &Path, mod_id: &str, main_class: &str, name: &str) -> Result<(), Error> {
    let config_path = path.join("src/main/resources/fabric.mod.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
    config["id"] = serde_json::Value::String(mod_id.to_string());
    config["name"] = serde_json::Value::String(name.to_string());
    config["description"] = serde_json::Value::String("".to_string());
    config["entrypoints"]["main"] = serde_json::Value::String(main_class.to_string());
    std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn update_mixin_config(path: &Path, mod_id: &str) -> Result<(), Error> {
    let config_path = path.join(format!("src/main/resources/{}.mixins.json", mod_id));
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;
    config["package"] = serde_json::Value::String(mod_id.to_string());
    std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn update_gradle_properties(path: &Path, group: &str, base_name: &str) -> Result<(), Error> {
    let config_path = path.join("gradle.properties");
    let mut config = std::fs::read_to_string(&config_path)?;
    config = config.replace("com.example", group);
    config = config.replace("fabric-example-mod", base_name);
    std::fs::write(config_path, config)?;
    Ok(())
}

pub fn create_mod(
    path: &Path,
    mod_id: &str,
    language: &Language,
    main_class: &str,
    name: &str,
) -> Result<(), Error> {
    // Clone the Kotlin example mod
    let template_url = match language {
        Language::Kotlin => "https://github.com/clabe45/fabric-example-mod-kotlin",
        Language::Java => "https://github.com/FabricMC/fabric-example-mod",
    };
    let global = git::Context::new(&None)?;
    global.git(&["clone", template_url, path.to_str().unwrap()])?;

    // Remove the .git directory
    let git_dir = path.join(".git");
    std::fs::remove_dir_all(git_dir)?;

    // Re-initialize the git repository
    let repo = git::Context::new(&Some(path))?;
    repo.git(&["init"])?;

    // Rename the package
    let old_package = "net.fabricmc.example";
    let new_package = main_class[..main_class.rfind('.').unwrap()].to_string();
    refactor::rename_package(path, language, &old_package, &new_package)?;

    // Rename the class
    let old_class = format!("{}.ExampleMod", &new_package);
    let new_class = main_class;
    refactor::rename_class(path, language, &old_class, &new_class)?;

    // Update the mixins config
    std::fs::rename(
        path.join("src/main/resources/modid.mixins.json"),
        path.join(format!("src/main/resources/{}.mixins.json", mod_id)),
    )?;
    update_mixin_config(path, mod_id)?;

    // Update the mod config
    update_mod_config(path, mod_id, main_class, name)?;

    // Update gradle.properties
    let group = &new_package[..new_package.rfind('.').unwrap()].to_string();
    let base_name = &new_package[new_package.rfind('.').unwrap() + 1..].to_string();
    update_gradle_properties(path, &group, &base_name)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{code::language::Language, fabric};

    #[rstest]
    #[case(Language::Java)]
    #[case(Language::Kotlin)]
    fn test_create_mod_creates_git_repo(#[case] language: Language) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_create_mod_creates_git_repo");
        fabric::create_mod(
            &path,
            "example-mod",
            &language,
            "net.fabricmc.example.ExampleMod",
            "Example Mod",
        )
        .unwrap();

        let git_dir = path.join(".git");
        assert!(git_dir.exists());
    }

    #[rstest]
    #[case(Language::Java)]
    #[case(Language::Kotlin)]
    fn test_create_mod_moves_entrypoint(#[case] language: Language) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_create_mod_moves_entrypoint");
        fabric::create_mod(
            &path,
            "example-mod2",
            &language,
            "net.fabricmc.example2.ExampleMod2",
            "Example Mod 2",
        )
        .unwrap();

        let entrypoint = path
            .join("src/main")
            .join(language.to_string())
            .join("net/fabricmc/example2/ExampleMod2.".to_string() + language.extension());

        assert!(entrypoint.exists());
    }

    #[rstest]
    #[case(Language::Java)]
    #[case(Language::Kotlin)]
    fn test_create_mod_renames_mixin_config(#[case] language: Language) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_create_mod_renames_mixin_config");
        fabric::create_mod(
            &path,
            "example-mod2",
            &language,
            "net.fabricmc.example3.ExampleMod2",
            "Example Mod 2",
        )
        .unwrap();

        let mixin_config = path.join("src/main/resources/example-mod2.mixins.json");
        assert!(mixin_config.exists());
    }

    #[rstest]
    #[case(Language::Java)]
    #[case(Language::Kotlin)]
    fn test_create_mod_updates_mod_config(#[case] language: Language) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test_create_mod_updates_mod_id");
        fabric::create_mod(
            &path,
            "example-mod2",
            &language,
            "net.fabricmc.example2.ExampleMod2",
            "Example Mod 2",
        )
        .unwrap();

        let mod_json = path.join("src/main/resources/fabric.mod.json");
        let contents = std::fs::read_to_string(mod_json).unwrap();
        let config: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(config["id"], "example-mod2");
        assert_eq!(config["name"], "Example Mod 2");
        assert_eq!(config["description"], "");
        assert_eq!(
            config["entrypoints"]["main"],
            "net.fabricmc.example2.ExampleMod2"
        );
    }

    #[rstest]
    #[case(Language::Java)]
    #[case(Language::Kotlin)]
    fn test_create_mod_updates_gradle_properties(#[case] language: Language) {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir
            .path()
            .join("test_create_mod_updates_gradle_properties");
        fabric::create_mod(
            &path,
            "example-mod2",
            &language,
            "net.fabricmc.example2.ExampleMod2",
            "Example Mod 2",
        )
        .unwrap();

        let gradle_properties = path.join("gradle.properties");
        let contents = std::fs::read_to_string(gradle_properties).unwrap();
        assert!(contents.contains("net.fabricmc"));
        assert!(contents.contains("example2"));
    }
}
