#!/usr/bin/env bash
# Weather fetcher using Open-Meteo APIs (no API key needed).
#
# Usage:
#   bash scripts/weather.sh "Chennai"
#   bash scripts/weather.sh "Chennai" "Mumbai" "Tokyo"
#
# Always exits with code 0 and prints JSON to stdout.
# Each city produces either a success or error object.
# Results are returned as a JSON array.

set -eo pipefail

GEOCODE_URL="https://geocoding-api.open-meteo.com/v1/search"
WEATHER_URL="https://api.open-meteo.com/v1/forecast"

# Weather code -> human-readable condition (WMO code table).
# Implemented as a function so the script works with macOS bash 3.2,
# which does not support associative arrays.
wmo_condition() {
    local code="$1"
    case "$code" in
        0) echo "Clear sky" ;;
        1) echo "Mainly clear" ;;
        2) echo "Partly cloudy" ;;
        3) echo "Overcast" ;;
        45) echo "Foggy" ;;
        48) echo "Icy Fog" ;;
        51) echo "Light drizzle" ;;
        53) echo "Moderate drizzle" ;;
        55) echo "Dense drizzle" ;;
        61) echo "Light rain" ;;
        63) echo "Moderate rain" ;;
        65) echo "Heavy rain" ;;
        71) echo "Light snow" ;;
        73) echo "Moderate snow" ;;
        75) echo "Heavy snow" ;;
        80) echo "Rain showers" ;;
        81) echo "Moderate rain showers" ;;
        82) echo "Violent rain showers" ;;
        95) echo "Thunderstorm" ;;
        96) echo "Thunderstorm with hail" ;;
        99) echo "Thunderstorm with heavy hail" ;;
        *) echo "Unknown (WMO code $code)" ;;
    esac
}

# Ensure jq is available; otherwise emit a single-element error array and exit 0.
if ! command -v jq >/dev/null 2>&1; then
    jq -n --arg city_query "unknown" --arg error "Missing dependency: install 'jq' (e.g. apt install jq / brew install jq) and try again." \
    '[{"city_query": $city_query, "error": $error}]'
    exit 0
fi

error_result() {
    local city="$1"
    local msg="$2"
    jq -n --arg city "$city" --arg msg "$msg" '{"city_query": $city, "error": $msg}'
}

# Fetch geocoding result for a city. On success prints a JSON object with
# name, country, admin1, latitude, longitude; on error prints the error string
# and returns non-zero.
geocode() {
    local city="$1"
    local encoded
    encoded=$(printf '%s' "$city" | jq -sRr @uri)

    local resp
    if ! resp=$(curl -sS --max-time 10 "${GEOCODE_URL}?name=${encoded}&count=1" 2>&1); then
        local rc=$?
        if [[ $rc -eq 28 ]]; then
            echo "Networking timed out for '$city'. Check your internet connection."
        else
            echo "Network error while looking up '$city': $resp"
        fi
        return 1
    fi

    # Sanity check: curl returned something, but we need valid JSON.
    if ! echo "$resp" | jq -e '.' >/dev/null 2>&1; then
        echo "Geocoding API returned invalid JSON for '$city'."
        return 1
    fi

    if [[ $(echo "$resp" | jq -r 'has("results")') != "true" ]]; then
        echo "Location '$city' not found. Check the spelling or try a nearby major city."
        return 1
    fi

    local count
    count=$(echo "$resp" | jq -r '.results | length')
    if [[ "$count" -eq 0 ]]; then
        echo "Location '$city' not found. Check the spelling or try a nearby major city."
        return 1
    fi

    echo "$resp" | jq -r '.results[0] | {name, country: (.country // ""), admin1: (.admin1 // ""), latitude, longitude}'
}

# Fetch weather for a given lat/lon. On success prints the raw forecast JSON;
# on error prints the error string and returns non-zero.
fetch_weather() {
    local city_query="$1"
    local lat="$2"
    local lon="$3"

    local resp
    if ! resp=$(curl -sS --max-time 10 \
        "${WEATHER_URL}?latitude=${lat}&longitude=${lon}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code,precipitation" 2>&1); then
        local rc=$?
        if [[ $rc -eq 28 ]]; then
            echo "Weather API timed out for '$city_query'. Check your internet connection."
        else
            echo "Network error fetching weather for '$city_query': $resp"
        fi
        return 1
    fi

    if ! echo "$resp" | jq -e '.' >/dev/null 2>&1; then
        echo "Weather API returned invalid JSON for '$city_query'."
        return 1
    fi

    echo "$resp"
}

get_weather_for_city() {
    local city="$1"

    local location
    local errv
    location=$(geocode "$city") || { errv="$location"; }

    if [[ -n "$errv" ]]; then
        error_result "$city" "$errv"
        return 0
    fi

    local name country admin1 latitude longitude
    name=$(echo "$location" | jq -r '.name')
    country=$(echo "$location" | jq -r '.country')
    admin1=$(echo "$location" | jq -r '.admin1')
    latitude=$(echo "$location" | jq -r '.latitude')
    longitude=$(echo "$location" | jq -r '.longitude')

    local raw
    raw=$(fetch_weather "$city" "$latitude" "$longitude") || { errv="$raw"; }

    if [[ -n "$errv" ]]; then
        error_result "$city" "$errv"
        return 0
    fi

    local current units
    current=$(echo "$raw" | jq -r '.current // empty')
    units=$(echo "$raw" | jq -r '.current_units // {}')

    if [[ -z "$current" ]]; then
        error_result "$city" "Weather API returned no current conditions. Try again later."
        return 0
    fi

    local code condition
    code=$(echo "$current" | jq -r '.weather_code // -1')
    condition=$(wmo_condition "$code")

    local time_utc temperature_c feels_like_c humidity_percent wind_speed_kmh precipitation_mm
    time_utc=$(echo "$current" | jq -r '.time // ""')
    temperature_c=$(echo "$current" | jq -r '.temperature_2m // null')
    feels_like_c=$(echo "$current" | jq -r '.apparent_temperature // null')
    humidity_percent=$(echo "$current" | jq -r '.relative_humidity_2m // null')
    wind_speed_kmh=$(echo "$current" | jq -r '.wind_speed_10m // null')
    precipitation_mm=$(echo "$current" | jq -r '.precipitation // null')

    local temperature_unit wind_speed_unit precipitation_unit
    temperature_unit=$(echo "$units" | jq -r '.temperature_2m // "°C"')
    wind_speed_unit=$(echo "$units" | jq -r '.wind_speed_10m // "km/h"')
    precipitation_unit=$(echo "$units" | jq -r '.precipitation // "mm"')

    jq -n \
        --arg city_query "$city" \
        --arg name "$name" \
        --arg region "$admin1" \
        --arg country "$country" \
        --argjson latitude "$latitude" \
        --argjson longitude "$longitude" \
        --arg time_utc "$time_utc" \
        --arg condition "$condition" \
        --argjson temperature_c "${temperature_c:-null}" \
        --argjson feels_like_c "${feels_like_c:-null}" \
        --argjson humidity_percent "${humidity_percent:-null}" \
        --argjson wind_speed_kmh "${wind_speed_kmh:-null}" \
        --argjson precipitation_mm "${precipitation_mm:-null}" \
        --arg temperature_unit "$temperature_unit" \
        --arg wind_speed_unit "$wind_speed_unit" \
        --arg precipitation_unit "$precipitation_unit" \
        '{
            city_query: $city_query,
            location: {
                name: $name,
                region: $region,
                country: $country,
                latitude: $latitude,
                longitude: $longitude
            },
            current: {
                time_utc: $time_utc,
                condition: $condition,
                temperature_c: $temperature_c,
                feels_like_c: $feels_like_c,
                humidity_percent: $humidity_percent,
                wind_speed_kmh: $wind_speed_kmh,
                precipitation_mm: $precipitation_mm
            },
            units: {
                temperature: $temperature_unit,
                wind_speed: $wind_speed_unit,
                precipitation: $precipitation_unit
            }
        }'
}

main() {
    local cities=()
    if [[ $# -eq 0 ]]; then
        cities=("Chennai")
    else
        for arg in "$@"; do
            local trimmed
            trimmed=$(echo "$arg" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
            if [[ -n "$trimmed" ]]; then cities+=("$trimmed"); fi
        done
    fi

    local results=()
    for city in "${cities[@]}"; do
        results+=("$(get_weather_for_city "$city")")
    done

    # Combine individual JSON objects into a single JSON array
    printf '%s\n' "${results[@]}" | jq -s '.'
}

main "$@"