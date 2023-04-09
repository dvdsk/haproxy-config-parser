use crate::config::Error;
use crate::sections::Line;

use super::super::sections::Section;

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Global {
    pub config: HashMap<String, Option<String>>,
    /// system user to run haproxy as
    pub user: Option<String>,
    /// system group to run haproxy as
    pub group: Option<String>,
}

impl<'a> TryFrom<&'a [Section<'a>]> for Global {
    type Error = Error<'a>;

    fn try_from(entries: &'a [Section<'_>]) -> Result<Self, Self::Error> {
        let global_entries: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e, Section::Global { .. }))
            .collect();

        if global_entries.len() > 1 {
            return Err(Error::MultipleGlobalEntries(global_entries));
        }

        let Some(Section::Global{ lines, .. }) = global_entries.first() else {
            return Ok(Global::default());
        };

        let mut config = HashMap::new();
        let mut user_lines = Vec::new();
        let mut group_lines = Vec::new();
        let mut other = Vec::new();

        for line in lines
            .iter()
            .filter(|l| !matches!(l, Line::Blank | Line::Comment(_)))
        {
            match line {
                Line::Config { key, value, .. } => {
                    let key = key.to_string();
                    let value = value.map(ToOwned::to_owned);
                    config.insert(key, value);
                }
                Line::SysUser { .. } => {
                    user_lines.push(line);
                }
                Line::Group { .. } => {
                    group_lines.push(line);
                }
                _other => other.push(_other),
            }
        }

        if !other.is_empty() {
            return Err(Error::WrongGlobalLines(other));
        }

        let (user, group) = extract_sys_user_group(user_lines, group_lines)?;

        Ok(Global {
            config,
            user,
            group,
        })
    }
}

fn extract_sys_user_group<'a>(
    mut user_lines: Vec<&'a Line>,
    mut group_lines: Vec<&'a Line>,
) -> Result<(Option<String>, Option<String>), Error<'a>> {
    if user_lines.len() > 1 {
        return Err(Error::MultipleSysUsers(user_lines));
    }
    if group_lines.len() > 1 {
        return Err(Error::MultipleSysGroups(group_lines));
    }

    let user = match user_lines.pop() {
        None => None,
        Some(Line::SysUser { name, .. }) => Some(name.to_string()),
        _other => unreachable!(),
    };

    let group = match group_lines.pop() {
        None => None,
        Some(line @ Line::Group { name, users, .. }) => {
            if !users.is_empty() {
                return Err(Error::SysGroupHasUsers(line));
            }
            Some(name.to_string())
        }
        _other => unreachable!(),
    };

    Ok((user, group))
}
