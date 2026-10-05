---
name: weather
description: "Fetches and displays current weather conditions for one or more cities using the Open-Meteo API (free, no API key needed). Make sure to use this skill whenever the user asks about weather, temperature, humidity, wind, rain, or outdoor conditions for any city or place - including multiple cities at once. Trigger on: get weather, check weather, what is the weather, weather forecast, temperature in [city], how is the weather in [place], is it raining in X, compare weather between X and Y, weather in X Y and Z - even if the word weather is not used but the intent is clearly about outdoor conditions. Default to Chennai if no location is mentioned."
---

# Weather Skill

This skill fetches live weather data using two free Open-Meteo APIs chained together:
1. **Geocoding API** - resolves a city name to lat/lon (so the user never has to type coordinates)
2. **Forecast API** - returns current conditions for those coordinates



## Running the script

Use a absolute path via the skill's root directory [${AGENT_SKILL_DIR}] - the agent runs skill commands
with the working directory already set there:

```bash
bash ${AGENT_SKILL_DIR}/scripts/weather.sh "Chennai"
bash ${AGENT_SKILL_DIR}/scripts/weather.sh "Chennai" "Mumbai" "Tokyo" "London"
```

If the user doesn't name a city, default to **Chennai**.

## Extracting cities from the user's message

Pull out every place name the user mentions - don't ask for clarification if you can infer them:

| User says | Cities to pass |
| --- | --- |
| "weather in Mumbai" | `"Mumbai"` |
| "compare Delhi and Bangalore" | `"Delhi"` `"Bangalore"` |
| "weather in Paris, New York and Tokyo" | `"Paris"` `"New York"` `"Tokyo"` |
| "check weather" (no city) | `"Chennai"` (default) |

## Output format

The script always exits with code 0 and prints a JSON array - one element per city. Using an array means partial failures don't hide successful results (e.g., 2 of 3 cities succeed).

The exact shape (success vs. error element, required fields, types) is defined in
[`${AGENT_SKILL_DIR}/references/output_schema.json`](${AGENT_SKILL_DIR}/references/output_schema.json) - read it if you need to validate
a response or aren't sure whether a field is required.

**Success element:**

```json
[
  {
    "city_query": "Chennai",
    "location": { "name": "Chennai", "region": "Tamil Nadu", "country": "India", "latitude": 13.08, "longitude": 80.27 },
    "current": {
      "time_utc": "2026-08-03T06:15",
      "condition": "Mainly clear",
      "temperature_c": 34.3,
      "feels_like_c": 39.0,
      "humidity_percent": 53,
      "wind_speed_kmh": 13.0,
      "precipitation_mm": 0.0
    },
    "units": { "temperature": "°C", "wind_speed": "km/h", "precipitation": "mm" }
  }
]

```

**Error element** (city not found, network issue, etc.):

```json
[
  { "city_query": "Xyzville", "error": "Location 'Xyzville' not found. Check the spelling or try a nearby major city." }
]

```

## Displaying results

If the script or any weather tool fails, immediately report the exact error to the user and
stop. Never invent, estimate, infer, or substitute weather data from seasonal patterns,
memory, or knowledge. Do not retry unrelated commands or provide general weather advice as
if it were current weather. At most one retry is allowed, and only when correcting an
obvious path or shell-formatting issue.

For **success** elements, use this block format - showing "feels like" alongside actual temperature matters most since it reflects real outdoor comfort:

```text
Weather in Chennai, Tamil Nadu, India
-------------------------------------
Condition:    Mainly clear
Temperature:  34.3°C (feels like 39.0°C)
Humidity:     53%
Wind:         13.0 km/h
Precipitation: 0.0 mm
Time (UTC):   2026-08-03T06:15

```

For **error** elements, show the error string directly - it's already user-friendly, no rewrapping needed:

```text
Could not fetch weather for 'Xyzville':
  Location 'Xyzville' not found. Check the spelling or try a nearby major city.

```

Separate multiple city blocks with a blank line.

## What the script handles so you don't have to

| Failure scenario | Script's response |
| --- | --- |
| `jq` not installed | Error element with install instruction |
| Network timeout (either API) | Error element with check internet connection |
| Non-200 HTTP status | Error element with the status code |
| City name not found by geocoder | Error element asking to check spelling |
| Malformed / non-JSON API response | Error element noting the parse failure |
| Mix of good and bad cities | Success elements for good, error elements for bad |
