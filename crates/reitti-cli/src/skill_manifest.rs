#[derive(Debug, PartialEq, Eq)]
pub struct Frontmatter<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub cli_version: &'a str,
    pub schema_version: u8,
}

pub fn parse(text: &str) -> Result<Frontmatter<'_>, String> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err("skill must start with YAML frontmatter".to_owned());
    }
    let mut name = None;
    let mut description = None;
    let mut cli_version = None;
    let mut schema_version = None;
    let mut closed = false;
    for line in lines.by_ref() {
        if line == "---" {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(": ") {
            let value = value.trim_matches('"');
            match key {
                "name" => name = unique(name, value, key)?,
                "description" => description = unique(description, value, key)?,
                "cli_version" => cli_version = unique(cli_version, value, key)?,
                "schema_version" => {
                    let parsed = value
                        .parse::<u8>()
                        .map_err(|_| "skill schema_version must be an integer".to_owned())?;
                    if schema_version.replace(parsed).is_some() {
                        return Err("duplicate skill frontmatter field 'schema_version'".to_owned());
                    }
                }
                _ => {}
            }
        }
    }
    if !closed || lines.next().is_none() {
        return Err("skill frontmatter must have a closing delimiter and a body".to_owned());
    }
    Ok(Frontmatter {
        name: name.ok_or_else(|| "skill frontmatter requires name".to_owned())?,
        description: description
            .ok_or_else(|| "skill frontmatter requires description".to_owned())?,
        cli_version: cli_version
            .ok_or_else(|| "skill frontmatter requires cli_version".to_owned())?,
        schema_version: schema_version
            .ok_or_else(|| "skill frontmatter requires schema_version".to_owned())?,
    })
}

fn unique<'a>(slot: Option<&'a str>, value: &'a str, key: &str) -> Result<Option<&'a str>, String> {
    if slot.is_some() {
        Err(format!("duplicate skill frontmatter field '{key}'"))
    } else {
        Ok(Some(value))
    }
}

pub fn verify<'a>(text: &'a str, expected_cli_version: &str) -> Result<Frontmatter<'a>, String> {
    let frontmatter = parse(text)?;
    if frontmatter.name != "reitti" {
        return Err("bundled skill name must be reitti".to_owned());
    }
    if frontmatter.description.len() > 1024 {
        return Err("bundled skill description exceeds 1024 bytes".to_owned());
    }
    if frontmatter.cli_version != expected_cli_version {
        return Err(format!(
            "bundled skill cli_version '{}' does not match package version '{}'; update the bundle in the same commit as every release bump",
            frontmatter.cli_version, expected_cli_version
        ));
    }
    if frontmatter.schema_version != 1 {
        return Err("bundled skill schema_version must be 1".to_owned());
    }
    Ok(frontmatter)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "---\nname: reitti\ndescription: Plan trips.\ncli_version: \"0.0.0\"\nschema_version: 1\n---\n# Reitti\n";

    #[test]
    fn verifies_the_leading_frontmatter() {
        assert!(verify(VALID, "0.0.0").is_ok());
        let body_only = "---\nname: reitti\ndescription: Plan trips.\ncli_version: \"old\"\nschema_version: 9\n---\ncli_version: \"0.0.0\"\nschema_version: 1\n";
        assert!(verify(body_only, "0.0.0").is_err());
    }

    #[test]
    fn rejects_cli_version_drift() {
        let error = verify(VALID, "1.0.0").unwrap_err();
        assert!(error.contains("does not match package version '1.0.0'"));
    }
}
