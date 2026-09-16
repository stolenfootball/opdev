import argparse
from . import label

parser = argparse.ArgumentParser(description="Print a parcel label")
parser.add_argument("recipient")
args = parser.parse_args()
try:
    print(label(args.recipient))
except ValueError as error:
    parser.error(str(error))
