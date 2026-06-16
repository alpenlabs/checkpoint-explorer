import { RpcCheckpointInfoCheckpointExp } from "../types/index";
const shortenIds = (
  value: string | null | undefined,
  startLength: number = 8,
  endLength: number = 6,
): string => {
  if (!value) return "N/A";
  if (value.length <= startLength + endLength) return value; // No need to shorten
  return `${value.slice(2, startLength)}...${value.slice(-endLength)}`;
};

function isRpcCheckpointInfo(data: any): data is RpcCheckpointInfoCheckpointExp[] {
  // TODO(STR-3793): Replace this any-based stale guard with typed API response parsing.
  return (
    Array.isArray(data) && // Ensure it's an array
    data.length > 0 && // Ensure the array is not empty
    typeof data[0] === "object" && // Check the first element is an object
    data[0] !== null &&
    "idx" in data[0] &&
    "l1_range" in data[0] &&
    Array.isArray(data[0].l1_range) &&
    data[0].l1_range.length === 2 &&
    "l2_range" in data[0] &&
    Array.isArray(data[0].l2_range) &&
    data[0].l2_range.length === 2 &&
    "l2_blockid" in data[0] &&
    typeof data[0].l2_blockid === "string"
  );
}
function reverseEndian(value: string | null | undefined): string {
  if (!value) return "N/A";
  const match = value.match(/.{2}/g);
  return match ? match.reverse().join("") : "N/A"; // Return fallback if match fails
}

function truncateTxid(value: string | null | undefined): string {
  console.log(value);
  if (!value) return "N/A";
  if (value === "N/A" || value === "-") return value;

  return `${value.substring(0, 4)}..${value.substring(value.length - 5)}`;
}
export { isRpcCheckpointInfo, reverseEndian, truncateTxid, shortenIds };
