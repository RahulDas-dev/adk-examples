//! Example 02: Agent with custom tools.
//!
//! Gives the agent two custom tools — `get_datetime` and `get_weather` — built
//! with `FunctionTool` and a `#[derive(JsonSchema)]` parameter struct so the
//! model knows how to call them.
//!
//! Setup: put these in a `.env` file at the project root (gitignored):
//! ```text
//! ANTHROPIC_API_KEY=sk-ant-...
//! ANTHROPIC_MODEL=claude-sonnet-5   # optional, defaults to claude-sonnet-5
//! ```
//!
//! Run:
//! ```sh
//! cargo run --example 02_custom_tools
//! ```
//!
//! Try asking it: "What time is it in UTC?" or "What's the weather in Tokyo?"

use adk_rust::Launcher;
use adk_rust::prelude::*;
use chrono::{DateTime, Local, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

// Parameter schemas

#[derive(Serialize, Deserialize, JsonSchema)]
struct GetDateTimeParams {
    /// Timezone: 'local' or 'utc' (optional, defaults to 'local')
    timezone: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
struct GetWeatherParams {
    /// City name (required). Supported: London, Tokyo, New York, Bangalore
    city: String,
}

// ============================================================================
// Tool 1: Get DateTime
// ============================================================================

async fn get_datetime_handler(_ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
    let timezone = args["timezone"].as_str().unwrap_or("local").to_lowercase();

    let response = match timezone.as_str() {
        "utc" => {
            let now: DateTime<Utc> = Utc::now();
            json!({
                "datetime": now.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                "timezone": "UTC",
                "unix_timestamp": now.timestamp(),
                "formatted": now.to_rfc3339()
            })
        }
        _ => {
            let now: DateTime<Local> = Local::now();
            json!({
                "datetime": now.format("%Y-%m-%d %H:%M:%S %z").to_string(),
                "timezone": "Local",
                "unix_timestamp": now.timestamp(),
                "formatted": now.to_rfc3339()
            })
        }
    };

    Ok(response)
}

// ============================================================================
// Tool 2: Get Weather
// ============================================================================

async fn get_weather_handler(_ctx: Arc<dyn ToolContext>, args: Value) -> Result<Value> {
    let city = args["city"]
        .as_str()
        .ok_or_else(|| AdkError::config("City parameter is required"))?;

    let weather = match city.to_lowercase().as_str() {
        "london" => json!({
            "city": "London",
            "temperature_celsius": 15,
            "temperature_fahrenheit": 59,
            "condition": "Cloudy",
            "humidity": 75,
            "wind_speed_kmh": 20,
            "description": "Overcast with a chance of light rain"
        }),
        "tokyo" => json!({
            "city": "Tokyo",
            "temperature_celsius": 22,
            "temperature_fahrenheit": 72,
            "condition": "Partly Cloudy",
            "humidity": 60,
            "wind_speed_kmh": 15,
            "description": "Pleasant with some clouds"
        }),
        "new york" | "newyork" => json!({
            "city": "New York",
            "temperature_celsius": 18,
            "temperature_fahrenheit": 64,
            "condition": "Sunny",
            "humidity": 55,
            "wind_speed_kmh": 10,
            "description": "Clear skies and warm"
        }),
        "bangalore" | "bengaluru" => json!({
            "city": "Bangalore",
            "temperature_celsius": 28,
            "temperature_fahrenheit": 82,
            "condition": "Partly Cloudy",
            "humidity": 65,
            "wind_speed_kmh": 12,
            "description": "Warm with occasional clouds"
        }),
        _ => json!({
            "city": city,
            "temperature_celsius": 20,
            "temperature_fahrenheit": 68,
            "condition": "Unknown",
            "humidity": 50,
            "wind_speed_kmh": 10,
            "description": format!("Weather data not available for {city}. Showing default values.")
        }),
    };

    Ok(weather)
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    let model_name = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-sonnet-5".to_string());

    let model = AnthropicClient::new(AnthropicConfig::new(&api_key, &model_name))?;

    let datetime_tool = FunctionTool::new(
        "get_datetime",
        "Get the current date and time",
        get_datetime_handler,
    )
    .with_parameters_schema::<GetDateTimeParams>();

    let weather_tool = FunctionTool::new(
        "get_weather",
        "Get weather information for a city",
        get_weather_handler,
    )
    .with_parameters_schema::<GetWeatherParams>();

    let agent = LlmAgentBuilder::new("tool_agent")
        .description("Claude with datetime and weather tools")
        .instruction(
            "You are a helpful assistant with tools for checking the time and weather. \
             Use them when asked.",
        )
        .model(Arc::new(model))
        .tool(Arc::new(datetime_tool))
        .tool(Arc::new(weather_tool))
        .build()?;

    Launcher::new(Arc::new(agent)).run().await?;
    Ok(())
}
