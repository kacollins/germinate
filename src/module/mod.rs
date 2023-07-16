use std::{collections::HashMap, fs, path::Path};
use toml::Table;

type Dependencies = HashMap<String, Vec<Module>>;
type Scripts = HashMap<String, Vec<Vec<String>>>;
type ThenCommands = Vec<Vec<String>>;
#[derive(Debug, Clone)]
pub struct Module {
    name: String,
    version: String,
    dev: bool,
    then: Option<ThenCommands>,
}

impl Module {
    pub fn new(name: String, version: String, dev: bool, then: Option<ThenCommands>) -> Self {
        Self {
            name,
            version,
            dev,
            then,
        }
    }
}

struct TomlTemplate {
    title: String,
    subfolders: Option<Vec<Vec<String>>>,
    scripts: Option<Scripts>,
    dependencies: Option<Dependencies>,
}

impl TomlTemplate {
    pub fn parse_deps(path: &Path) -> Option<Dependencies> {
        let template_str = fs::read_to_string(path).expect("Error reading file");
        let table = template_str.parse::<Table>().expect("Error parsing toml");
        let deps = table["deps"]
            .as_table()
            .expect("Error parsing dependencies");

        let npm_deps: Vec<_> = if deps.contains_key("npm") {
            deps["npm"]
                .as_array()
                .expect("Error parsing npm dependencies")
                .iter()
                .map(|dep| {
                    let dep = dep.as_table().expect("Error parsing dep");
                    let name = dep["name"].as_str().expect("Error parsing name");

                    //TODO: add semver crate to allow for parsing semver ranges
                    let version = if dep.contains_key("version") {
                        dep["version"].as_str().expect("Error parsing version")
                    } else {
                        "latest"
                    };

                    let dev = if dep.contains_key("dev") {
                        dep["dev"].as_bool().expect("Error parsing dev")
                    } else {
                        false
                    };

                    let then = if dep.contains_key("then") {
                        let cmds = dep["then"]
                            .as_array()
                            .expect("Error parsing then array")
                            .iter()
                            .map(|arr| arr.as_array().expect("Error parsing cmd"))
                            .map(|cmd_arr| {
                                cmd_arr
                                    .iter()
                                    .map(|arg| arg.as_str().expect("Error parsing arg").to_string())
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        Some(cmds)
                    } else {
                        None
                    };
                    Module::new(name.to_string(), version.to_string(), dev, then)
                })
                .collect()
        } else {
            Vec::new()
        };

        let mut deps = HashMap::new();
        deps.insert("npm".to_string(), npm_deps);
        Some(deps)
    }
}

pub struct NpmDeps {
    deps: Vec<Module>,
}

impl NpmDeps {
    pub fn new() -> Self {
        Self { deps: Vec::new() }
    }

    pub fn add(&mut self, module: Module) {
        self.deps.push(module);
    }

    pub fn get(&self) -> Vec<Module> {
        self.deps.clone()
    }
}

// TODO add builders for composer and cargo deps
// ? refactor TomlTemplate construction to deserialize traits for toml? (maybe not worth it)
#[cfg(test)]
pub mod tests {

    use super::*;

    #[test]
    fn test_parse_toml() {
        let path = Path::new("templates/ssrjs/ssrjs.toml");
        let template_str = fs::read_to_string(path).expect("Error reading file");
        let table = template_str.parse::<Table>().expect("Error parsing toml");
        assert_eq!(table["title"].as_str(), Some("ssrjs"));
    }

    #[test]
    fn extract_npm_deps() {
        let path = Path::new("test/__mocks__/_test.toml");
        let parsed_deps = TomlTemplate::parse_deps(path).expect("Error parsing deps");
        assert!(parsed_deps.contains_key("npm"));

        let npm_deps = &parsed_deps["npm"];
        assert!(npm_deps.iter().any(|dep| dep.name == "test_npm_dep_min"));
        assert!(npm_deps
            .iter()
            .any(|dep| dep.name == "test_npm_dev_dep_full"));

        let full_dev_dep = npm_deps
            .iter()
            .find(|dep| dep.name == "test_npm_dev_dep_full")
            .expect("Error finding dep");
        assert_eq!(full_dev_dep.version, "^1.0.0");
        assert_eq!(full_dev_dep.dev, true);

        let then_cmds = full_dev_dep.then.as_ref().unwrap();
        assert_eq!(then_cmds.len(), 2);
        assert_eq!(then_cmds[0].len(), 1);
        assert_eq!(then_cmds[1].len(), 3);
        assert_eq!(then_cmds[0][0], "naked_command");
        assert_eq!(then_cmds[1][0], "command_with_args");
        assert_eq!(then_cmds[1][1], "arg1");
        assert_eq!(then_cmds[1][2], "arg2");
    }
}