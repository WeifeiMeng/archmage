use super::Store;
use crate::{error::{Error, Result}, team::{Team, TeamGraph}};
use rusqlite::{OptionalExtension, params};

fn validated_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(Error::Preflight("team name must be 1–80 characters".into()));
    }
    Ok(name)
}

impl Store {
    pub fn teams(&self) -> Result<Vec<Team>> {
        let mut query = self.db.prepare(
            "SELECT id,name,revision,graph_json FROM teams ORDER BY CASE id WHEN 'default' THEN 0 ELSE 1 END,name"
        )?;
        let rows = query.query_map([], |row| {
            let graph: String = row.get(3)?;
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, graph))
        })?.collect::<rusqlite::Result<Vec<(String,String,i64,String)>>>()?;
        rows.into_iter().map(|(id,name,revision,graph)| {
            let graph: TeamGraph = serde_json::from_str(&graph)
                .map_err(|error| Error::Preflight(format!("invalid stored team graph: {error}")))?;
            graph.validate()?;
            Ok(Team { id, name, revision, graph })
        }).collect()
    }

    pub fn team(&self, id: &str) -> Result<Team> {
        let row: Option<(String,String,i64,String)> = self.db.query_row(
            "SELECT id,name,revision,graph_json FROM teams WHERE id=?1", [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional()?;
        let (id,name,revision,graph) = row.ok_or_else(|| Error::Preflight(format!("unknown team: {id}")))?;
        let graph: TeamGraph = serde_json::from_str(&graph)
            .map_err(|error| Error::Preflight(format!("invalid stored team graph: {error}")))?;
        graph.validate()?;
        Ok(Team { id, name, revision, graph })
    }

    pub fn create_team(&mut self, name: &str, graph: &TeamGraph) -> Result<Team> {
        let name = validated_name(name)?;
        graph.validate()?;
        let id = uuid::Uuid::new_v4().to_string();
        self.db.execute(
            "INSERT INTO teams(id,name,graph_json) VALUES(?1,?2,?3)",
            params![id,name,serde_json::to_string(graph).map_err(|error| Error::Preflight(error.to_string()))?],
        )?;
        self.team(&id)
    }

    pub fn save_team(&mut self, id: &str, revision: i64, name: &str, graph: &TeamGraph) -> Result<Team> {
        let name = validated_name(name)?;
        graph.validate()?;
        let changed = self.db.execute(
            "UPDATE teams SET name=?3,graph_json=?4,revision=revision+1,
                    updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE id=?1 AND revision=?2",
            params![id,revision,name,serde_json::to_string(graph).map_err(|error| Error::Preflight(error.to_string()))?],
        )?;
        if changed != 1 { return Err(Error::Preflight("team changed; refresh and retry".into())); }
        self.team(id)
    }
}
