import argparse
import json
from .receipt import receipt

parser = argparse.ArgumentParser()
parser.add_argument("weight", type=int)
parser.add_argument("zone", choices=["local", "national"])
args = parser.parse_args()
print(json.dumps(receipt(args.weight, args.zone)))
