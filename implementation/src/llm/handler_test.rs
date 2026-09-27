use super::Inner;

#[tokio::test]
async fn budget_exhaustion_from_lua_callback_becomes_timeout_sentinel() -> anyhow::Result<()> {
    let direct = Inner::try_catch_budget_exhausted(anyhow::Error::from(mlua::Error::external(
        crate::common::BudgetExhausted,
    )))?;
    assert_eq!(direct.consumed_gen, primitive_types::U256::MAX);

    let lua = mlua::Lua::new();
    lua.globals().set(
        "exhaust",
        lua.create_async_function(|_, ()| async {
            Err::<(), mlua::Error>(mlua::Error::external(crate::common::BudgetExhausted))
        })?,
    )?;

    let call = lua
        .load("return function() exhaust() end")
        .eval::<mlua::Function>()?;
    let err = call
        .call_async::<()>(())
        .await
        .expect_err("budget must exhaust");
    assert!(
        matches!(
            err,
            mlua::Error::CallbackError { ref cause, .. }
                if matches!(cause.as_ref(), mlua::Error::ExternalError(ext)
                    if ext.downcast_ref::<crate::common::BudgetExhausted>().is_some())
        ),
        "expected Lua callback to wrap BudgetExhausted, got {err:?}"
    );

    let answer = Inner::try_catch_budget_exhausted(anyhow::Error::from(err))?;
    assert_eq!(answer.consumed_gen, primitive_types::U256::MAX);
    assert!(matches!(
        answer.data,
        genvm_modules_interfaces::llm::PromptAnswerData::Text(ref text) if text.is_empty()
    ));
    Ok(())
}
