from fastmcp import FastMCP

mcp = FastMCP(name="Fruits seller", version="1.0.0")

FRUIT_INVENTORY = {
    "apple": {
        "Pricing": {
            "piece": {"price": 1.0, "availability": 100},
            "kg": {"price": 3.0, "availability": 50},
            "box": {"price": 20.0, "availability": 10},
        }
    },
    "banana": {
        "Pricing": {
            "piece": {"price": 0.5, "availability": 200},
            "kg": {"price": 2.0, "availability": 80},
            "box": {"price": 15.0, "availability": 15},
        }
    },
    "orange": {
        "Pricing": {
            "piece": {"price": 0.8, "availability": 150},
            "kg": {"price": 2.5, "availability": 60},
            "box": {"price": 18.0, "availability": 12},
        }
    },
    "grape": {
        "Pricing": {
            "piece": {"price": 1.2, "availability": 120},
            "kg": {"price": 4.0, "availability": 40},
            "box": {"price": 25.0, "availability": 8},
        }
    },
    "mango": {
        "Pricing": {
            "piece": {"price": 2.0, "availability": 80},
            "kg": {"price": 5.0, "availability": 30},
            "box": {"price": 30.0, "availability": 5},
        }
    },
    "strawberry": {
        "Pricing": {
            "piece": {"price": 0.3, "availability": 300},
            "kg": {"price": 1.5, "availability": 100},
            "box": {"price": 10.0, "availability": 20},
        }
    },
    "watermelon": {
        "Pricing": {
            "piece": {"price": 3.0, "availability": 50},
            "kg": {"price": 2.0, "availability": 100},
            "box": {"price": 25.0, "availability": 10},
        }
    },
    "pineapple": {
        "Pricing": {
            "piece": {"price": 2.5, "availability": 60},
            "kg": {"price": 4.5, "availability": 40},
            "box": {"price": 35.0, "availability": 8},
        }
    },
    "kiwi": {
        "Pricing": {
            "piece": {"price": 1.5, "availability": 90},
            "kg": {"price": 6.0, "availability": 20},
            "box": {"price": 40.0, "availability": 4},
        }
    },
    "peach": {
        "Pricing": {
            "piece": {"price": 1.8, "availability": 70},
            "kg": {"price": 5.5, "availability": 25},
            "box": {"price": 32.0, "availability": 6},
        }
    },
    "cherry": {
        "Pricing": {
            "piece": {"price": 0.2, "availability": 400},
            "kg": {"price": 3.5, "availability": 90},
            "box": {"price": 22.0, "availability": 18},
        }
    },
    "pomegranate": {
        "Pricing": {
            "piece": {"price": 1.0, "availability": 110},
            "kg": {"price": 4.0, "availability": 35},
            "box": {"price": 28.0, "availability": 7},
        }
    },
    "blueberry": {
        "Pricing": {
            "piece": {"price": 0.4, "availability": 250},
            "kg": {"price": 3.0, "availability": 70},
            "box": {"price": 20.0, "availability": 15},
        }
    },
}


@mcp.tool()
def get_fruit_pricing(fruit_name: str, unit: str = "all") -> dict:
    """
    Get the pricing and availability of a specific fruit.

    Args:
        fruit_name (str): The name of the fruit.
        unit (str): The unit of measurement ("piece", "kg", "box", or "all").

    Returns:
        dict: A dictionary containing the pricing and availability information for the specified fruit.
    """
    fruit_name = fruit_name.lower()
    unit = unit.lower()

    if fruit_name not in FRUIT_INVENTORY:
        available_units = list(
            FRUIT_INVENTORY.get(fruit_name, {}).get("Pricing", {}).keys()
        )
        return {
            "error": f"Fruit '{fruit_name}' not found in inventory.",
            "available_units": available_units,
        }
    fruit_info = FRUIT_INVENTORY.get(fruit_name, {})

    if unit == "all":
        return fruit_info
    if unit not in fruit_info.get("Pricing", {}):
        available_units = list(fruit_info.get("Pricing", {}).keys())
        return {
            "error": f"Unit '{unit}' not found for fruit '{fruit_name}'.",
            "available_units": available_units,
        }
    else:
        return {
            "fruit": fruit_name,
            "unit": unit,
            "price": fruit_info["Pricing"][unit]["price"],
            "availability": fruit_info["Pricing"][unit]["availability"],
        }


@mcp.tool()
def calculate_total_cost(order: list[dict[str, str | float | int]]) -> float:
    """
    Calculate the total cost for a given fruit, unit, and quantity.

    Args:
        order (list[dict[str, str|float|int]]): A list of dictionaries containing fruit_name, unit, and quantity
        example: [{"fruit_name": "apple", "unit": "kg", "quantity": 2}, {"fruit_name": "banana", "unit": "piece", "quantity": 5}]
    Returns:
        float: The total cost of the order.
    """
    if not order:
        return {"error": "Order is empty. Please provide a valid order."}

    breakdown = []
    total_cost = 0.0
    errors = []

    for item in order:
        fruit_name = item.get("fruit_name")
        unit = item.get("unit")
        quantity = item.get("quantity")

        if fruit_name not in FRUIT_INVENTORY:
            errors.append({"error": f"Fruit '{fruit_name}' not found in inventory."})
            continue

        if unit not in FRUIT_INVENTORY[fruit_name]["Pricing"]:
            available_units = list(FRUIT_INVENTORY[fruit_name]["Pricing"].keys())
            errors.append({
                "error": f"Unit '{unit}' not found for fruit '{fruit_name}'.",
                "available_units": available_units,
            })
            continue

        if not all([fruit_name, unit, quantity is not None]):
            errors.append({
                "error": "Each order item must include 'fruit_name', 'unit', and 'quantity'."
            })
            continue

        fruit_info = get_fruit_pricing(fruit_name, unit)
        if "error" in fruit_info:
            errors.append(fruit_info)
            continue

        total_cost += fruit_info["price"] * quantity
        breakdown.append({
            "fruit_name": fruit_name,
            "unit": unit,
            "quantity": quantity,
            "price_per_unit": fruit_info["price"],
            "cost": fruit_info["price"] * quantity,
        })

    result = {"total_cost": total_cost, "breakdown": breakdown, "currency": "INR"}

    if errors:
        result["errors"] = errors

    return result


@mcp.resource("fruits://catalog")
def get_catalog() -> str:
    """
    Get the catalog of available fruits and their pricing.

    Returns:
        str: A string representation of the fruit catalog.
    """
    catalog = "Available Fruits and Pricing:\n"
    for fruit, details in FRUIT_INVENTORY.items():
        catalog += f"\n{fruit.capitalize()}:\n"
        for unit, pricing in details["Pricing"].items():
            catalog += f"  {unit.capitalize()}: Price = {pricing['price']} INR, Availability = {pricing['availability']}\n"
    return catalog


@mcp.prompt()
def recommend_fruit(budget: float, preferences: str = "mixed") -> str:
    """
    Recommend fruits based on the user's budget and preferences.

    Args:
        budget (float): The user's budget for purchasing fruits.
        preferences (str): The user's preferences for fruit types ("sweet", "citrus", "tropical", or "mixed").

    Returns:
        str: A recommendation of fruits that fit within the budget and match the preferences.
    """
    recommendations = []
    total_cost = 0.0

    for fruit, details in FRUIT_INVENTORY.items():
        if preferences != "mixed" and preferences not in fruit:
            continue
        for unit, pricing in details["Pricing"].items():
            if pricing["price"] <= budget:
                recommendations.append(
                    f"{fruit.capitalize()} ({unit.capitalize()}): Price = {pricing['price']} INR, Availability = {pricing['availability']}"
                )
                total_cost += pricing["price"]

    if not recommendations:
        return f"No fruits found within the budget of {budget} INR and preferences '{preferences}'."

    return (
        "Recommended Fruits:\n"
        + "\n".join(recommendations)
        + f"\nTotal Cost: {total_cost} INR"
    )


mcp.run(transport="streamable-http", port=3000)
