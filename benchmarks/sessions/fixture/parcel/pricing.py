RATES = {"local": 200, "national": 500}


def quote(weight_grams, zone):
    if type(weight_grams) is not int or weight_grams <= 0:
        raise ValueError("weight must be a positive integer")
    if zone not in RATES:
        raise ValueError("unknown zone")
    return (weight_grams // 1000) * RATES[zone]
