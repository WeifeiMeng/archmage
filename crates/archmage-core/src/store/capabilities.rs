use super::Store;
use crate::{
    capability::{BUILTIN_PROMPT_VERSION, CapabilityConfiguration, CapabilityConfigurationInput},
    error::{Error, Result},
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl Store {
    pub fn capability_configurations(&self) -> Result<Vec<CapabilityConfiguration>> {
        let mut query = self.db.prepare(
            "SELECT c.key,c.display_name,c.description,c.extra_instructions,
                    b.agent_instance_id,c.revision,c.skills_json,c.tools_json
             FROM capability_configurations c
             JOIN capability_bindings b ON b.capability=c.key
             ORDER BY CASE c.key WHEN 'designer' THEN 0 WHEN 'coder' THEN 1
                                   WHEN 'tester' THEN 2 ELSE 3 END",
        )?;
        Ok(query
            .query_map([], |row| {
                Ok(CapabilityConfiguration {
                    key: row.get(0)?,
                    display_name: row.get(1)?,
                    description: row.get(2)?,
                    extra_instructions: row.get(3)?,
                    agent_instance_id: row.get(4)?,
                    revision: row.get(5)?,
                    skills: serde_json::from_str(&row.get::<_, String>(6)?).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    tools: serde_json::from_str(&row.get::<_, String>(7)?).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            7,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    prompt_version: BUILTIN_PROMPT_VERSION.into(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_capability_configuration(
        &mut self,
        input: &CapabilityConfigurationInput,
    ) -> Result<()> {
        input.validate()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let instance: Option<(String, bool)> = tx
            .query_row(
                "SELECT provider_kind,enabled FROM agent_instances WHERE id=?1",
                [&input.agent_instance_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if !matches!(instance, Some((ref kind, true)) if kind.parse().is_ok_and(crate::agent::ProviderKind::supported))
        {
            return Err(Error::Preflight(
                "capability requires an enabled supported agent instance".into(),
            ));
        }
        let skills_json = serde_json::to_string(&input.skills)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let tools_json = serde_json::to_string(&input.tools)
            .map_err(|error| Error::Preflight(error.to_string()))?;
        let changed = tx.execute(
            "UPDATE capability_configurations SET display_name=?2,description=?3,
                    extra_instructions=?4,skills_json=?6,tools_json=?7,revision=revision+1,
                    updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE key=?1 AND revision=?5",
            params![
                input.key,
                input.display_name.trim(),
                input.description.trim(),
                input.extra_instructions,
                input.revision,
                skills_json,
                tools_json,
            ],
        )?;
        if changed != 1 {
            return Err(Error::Preflight(
                "capability configuration changed; refresh and retry".into(),
            ));
        }
        tx.execute(
            "UPDATE capability_bindings SET agent_instance_id=?2,
                    updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE capability=?1",
            params![input.key, input.agent_instance_id],
        )?;
        tx.commit()?;
        Ok(())
    }
}
