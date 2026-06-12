function sum(a: number, b: number): number {
  return a + b;
}

function greet(name: string): string {
  return `Hello, ${name}!`;
}

function makeUser(id: number, name: string) {
  return { id, name, tags: ["admin", "active"] };
}

function makeBigArray(): number[] {
  return Array.from({ length: 1000 }, (_, i) => i * 2);
}

function makeCircular() {
  const obj: any = { name: "root" };
  obj.self = obj;
  return obj;
}

function failingFn(): number {
  throw new Error("Something went wrong!");
}

async function asyncFetch(id: number): Promise<{ id: number; data: string }> {
  await Bun.sleep(50);
  return { id, data: `payload-${id}` };
}

async function main() {
  console.log("=== Test starting ===");

  const a = sum(2, 3);
  console.log("sum:", a);

  const g = greet("World");
  console.log("greet:", g);

  const u = makeUser(1, "Ali");
  console.log("user:", u);

  const big = makeBigArray();
  console.log("big array length:", big.length);

  const c = makeCircular();
  console.log("circular:", c.name);

  try {
    failingFn();
  } catch (e) {
    console.log("caught:", (e as Error).message);
  }

  const fetched = await asyncFetch(42);
  console.log("async:", fetched);

  console.log("=== Test done ===");
}

main();
