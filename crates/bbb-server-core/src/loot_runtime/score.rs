use super::*;

impl Eval<'_> {
    pub(super) fn score_value(
        &self,
        target: &ScoreTarget,
        objective: &bbb_protocol::JavaString,
        scale: f32,
        path: &str,
    ) -> Result<f32> {
        let score = match target {
            ScoreTarget::Fixed(name) => self.environment(path)?.score(name, objective),
            ScoreTarget::Context(param) => {
                let entity = self.context.entity(*param);
                if !entity.present() {
                    return Ok(0.0);
                }
                self.environment(path)?.entity_score(
                    entity,
                    objective,
                    self.context.entity_bodies(),
                )
            }
        };
        Ok(self
            .at(path, score)?
            .map_or(0.0, |score| score as f32 * scale))
    }

    pub(super) fn entity_scores(
        &mut self,
        param: LootContextParam,
        scores: &[(bbb_protocol::JavaString, IntRange)],
        path: &str,
    ) -> Result<bool> {
        let entity = self.context.entity(param);
        if !entity.present() {
            return Ok(false);
        }
        for (objective, range) in scores {
            let score = self.at(
                path,
                self.environment(path)?.entity_score(
                    entity,
                    objective,
                    self.context.entity_bodies(),
                ),
            )?;
            let Some(score) = score else {
                return Ok(false);
            };
            if !self.int_range_test(range, score, &format!("{path}.scores.{objective}"))? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
