from .pricing import quote


def receipt(weight_grams, zone):
    return {"weight_grams": weight_grams, "zone": zone,
            "price_cents": quote(weight_grams, zone)}
