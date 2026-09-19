// Working source snapshot W2; not the published 1.0.0 implementation.
export function capacityLabel(total, used, prepare) {
  prepare();
  return `${total - used} slots`;
}
