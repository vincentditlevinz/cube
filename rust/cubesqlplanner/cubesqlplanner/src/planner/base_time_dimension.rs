use super::query_tools::QueryTools;
use super::sql_evaluator::{Compiler, MemberSymbol, TimeDimensionSymbol};
use super::BaseDimension;
use super::Granularity;
use super::GranularityHelper;
use super::QueryDateTime;
use super::{evaluate_with_context, BaseMember, BaseMemberHelper, VisitorContext};
use crate::planner::sql_templates::PlanSqlTemplates;
use cubenativeutils::CubeError;
use std::rc::Rc;

pub struct BaseTimeDimension {
    dimension: Rc<BaseDimension>,
    member_evaluator: Rc<MemberSymbol>,
    query_tools: Rc<QueryTools>,
    granularity: Option<String>,
    granularity_obj: Option<Granularity>,
    date_range: Option<Vec<String>>,
    default_alias: String,
    alias_suffix: String,
}

impl BaseMember for BaseTimeDimension {
    fn to_sql(
        &self,
        context: Rc<VisitorContext>,
        templates: &PlanSqlTemplates,
    ) -> Result<String, CubeError> {
        evaluate_with_context(
            &self.member_evaluator,
            self.query_tools.clone(),
            context,
            templates,
        )
    }

    fn alias_name(&self) -> String {
        self.default_alias.clone()
    }

    fn member_evaluator(&self) -> Rc<MemberSymbol> {
        self.member_evaluator.clone()
    }
    fn full_name(&self) -> String {
        self.member_evaluator.full_name()
    }

    fn as_base_member(self: Rc<Self>) -> Rc<dyn BaseMember> {
        self.clone()
    }

    fn cube_name(&self) -> &String {
        &self.dimension.cube_name()
    }

    fn name(&self) -> &String {
        &self.dimension.name()
    }

    /*     fn alias_suffix(&self) -> Option<String> {
        Some(self.alias_suffix.clone())
    } */
}

impl BaseTimeDimension {
    pub fn try_new_from_td_symbol(
        query_tools: Rc<QueryTools>,
        td_symbol: Rc<TimeDimensionSymbol>,
    ) -> Result<Rc<Self>, CubeError> {
        let dimension =
            BaseDimension::try_new_required(td_symbol.base_symbol().clone(), query_tools.clone())?;
        let granularity = td_symbol.granularity().clone();
        let granularity_obj = td_symbol.granularity_obj().clone();
        let date_range = td_symbol.date_range_vec();
        let alias_suffix = td_symbol.alias_suffix();
        let default_alias = BaseMemberHelper::default_alias(
            &dimension.cube_name(),
            &dimension.name(),
            &Some(alias_suffix.clone()),
            query_tools.clone(),
        )?;
        let member_evaluator = MemberSymbol::new_time_dimension(td_symbol.clone());

        Ok(Rc::new(Self {
            dimension,
            query_tools,
            granularity,
            granularity_obj,
            date_range,
            alias_suffix,
            default_alias,
            member_evaluator,
        }))
    }

    pub fn try_new_required(
        query_tools: Rc<QueryTools>,
        member_evaluator: Rc<MemberSymbol>,
        compiler: &mut Compiler,
        granularity: Option<String>,
        date_range: Option<Vec<String>>,
    ) -> Result<Rc<Self>, CubeError> {
        let alias_suffix = if let Some(granularity) = &granularity {
            granularity.clone()
        } else {
            "day".to_string()
        };

        let dimension =
            BaseDimension::try_new_required(member_evaluator.clone(), query_tools.clone())?;
        let default_alias = BaseMemberHelper::default_alias(
            &dimension.cube_name(),
            &dimension.name(),
            &Some(alias_suffix.clone()),
            query_tools.clone(),
        )?;

        let granularity_obj = GranularityHelper::make_granularity_obj(
            query_tools.cube_evaluator().clone(),
            compiler,
            query_tools.timezone().clone(),
            &dimension.cube_name(),
            &dimension.name(),
            granularity.clone(),
        )?;

        let date_range_tuple = if let Some(date_range) = &date_range {
            assert_eq!(date_range.len(), 2);
            Some((date_range[0].clone(), date_range[1].clone()))
        } else {
            None
        };
        let member_evaluator = MemberSymbol::new_time_dimension(TimeDimensionSymbol::new(
            member_evaluator.clone(),
            granularity.clone(),
            granularity_obj.clone(),
            date_range_tuple,
        ));
        Ok(Rc::new(Self {
            dimension,
            query_tools,
            granularity,
            granularity_obj,
            date_range,
            alias_suffix,
            default_alias,
            member_evaluator,
        }))
    }

    pub fn change_granularity(
        &self,
        new_granularity: Option<String>,
    ) -> Result<Rc<Self>, CubeError> {
        let evaluator_compiler_cell = self.query_tools.evaluator_compiler().clone();
        let mut evaluator_compiler = evaluator_compiler_cell.borrow_mut();

        let new_granularity_obj = GranularityHelper::make_granularity_obj(
            self.query_tools.cube_evaluator().clone(),
            &mut evaluator_compiler,
            self.query_tools.timezone(),
            &self.dimension.name(),
            &self.dimension.cube_name(),
            new_granularity.clone(),
        )?;
        let date_range_tuple = if let Some(date_range) = &self.date_range {
            assert_eq!(date_range.len(), 2);
            Some((date_range[0].clone(), date_range[1].clone()))
        } else {
            None
        };
        let member_evaluator = MemberSymbol::new_time_dimension(TimeDimensionSymbol::new(
            self.dimension.member_evaluator(),
            new_granularity.clone(),
            new_granularity_obj.clone(),
            date_range_tuple,
        ));
        Ok(Rc::new(Self {
            dimension: self.dimension.clone(),
            granularity_obj: new_granularity_obj,
            query_tools: self.query_tools.clone(),
            granularity: new_granularity,
            date_range: self.date_range.clone(),
            alias_suffix: self.alias_suffix.clone(),
            default_alias: self.default_alias.clone(),
            member_evaluator,
        }))
    }

    pub fn get_granularity(&self) -> Option<String> {
        self.granularity.clone()
    }

    pub fn get_granularity_obj(&self) -> &Option<Granularity> {
        &self.granularity_obj
    }

    pub fn resolved_granularity(&self) -> Result<Option<String>, CubeError> {
        let res = if let Some(granularity_obj) = &self.granularity_obj {
            Some(granularity_obj.resolved_granularity()?)
        } else {
            None
        };
        Ok(res)
    }

    pub fn has_granularity(&self) -> bool {
        self.granularity.is_some()
    }

    pub fn get_date_range(&self) -> Option<Vec<String>> {
        self.date_range.clone()
    }

    pub fn get_range_for_time_series(&self) -> Result<Option<Vec<String>>, CubeError> {
        let res = if let Some(date_range) = &self.date_range {
            if date_range.len() != 2 {
                return Err(CubeError::user(format!(
                    "Invalid date range: {:?}",
                    date_range
                )));
            } else {
                if let Some(granularity_obj) = &self.granularity_obj {
                    if !granularity_obj.is_predefined_granularity() {
                        let tz = self.query_tools.timezone();
                        let start = QueryDateTime::from_date_str(tz, &date_range[0])?;
                        let start = granularity_obj.align_date_to_origin(start)?;
                        let end = QueryDateTime::from_date_str(tz, &date_range[1])?;

                        Some(vec![start.to_string(), end.to_string()])
                    } else {
                        Some(vec![date_range[0].clone(), date_range[1].clone()])
                    }
                } else {
                    Some(vec![date_range[0].clone(), date_range[1].clone()])
                }
            }
        } else {
            None
        };
        Ok(res)
    }

    pub fn base_dimension(&self) -> Rc<BaseDimension> {
        self.dimension.clone()
    }

    pub fn base_member_evaluator(&self) -> Rc<MemberSymbol> {
        self.dimension.member_evaluator()
    }

    pub fn unescaped_alias_name(&self) -> String {
        let granularity = if let Some(granularity) = &self.granularity {
            granularity
        } else {
            "day"
        };

        self.query_tools
            .alias_name(&format!("{}_{}", self.dimension.dimension(), granularity))
    }
}
