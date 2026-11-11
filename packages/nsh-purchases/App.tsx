import { mount, signal, Text, View } from "nsh";
import { type EntitlementsState, Purchases, PurchasesError, useEntitlements } from "./src";

/** The device's own store, through the Rust crate. */
const purchases = new Purchases();

const message = signal("nothing yet");

// The demo connects once at start, so both rows can answer; with no host the reads say their code.
void purchases.initialize().catch(() => undefined);

function describe(state: EntitlementsState): string {
  if (state.status === "checking") return "entitlements checking";
  if (state.status === "failed") return `entitlements failed ${state.error.code}`;
  const ids = state.entitlements.map((entitlement) => entitlement.productId);
  return `entitlements ${ids.join(",") || "none"}`;
}

function codeOf(error: unknown): string {
  return error instanceof PurchasesError ? error.code : "unknown error";
}

function App() {
  const entitlements = useEntitlements({ purchases });
  const buy = () => {
    purchases
      .initialize()
      .then(() => purchases.purchase("pro_unlock"))
      .then(
        (purchase) => message.set(`bought ${purchase.productId} (${purchase.state})`),
        (error: unknown) => message.set(codeOf(error)),
      );
  };
  return (
    <View class="flex-1 gap-3 p-4">
      <Text testID="title" class="text-foreground">nsh-purchases</Text>
      <Text testID="entitlements" class="text-foreground">{describe(entitlements.state())}</Text>
      <Text testID="message" class="text-foreground">{message()}</Text>
      <Text testID="buy" role="button" onPress={buy} class="rounded-(--radius) bg-muted px-3 py-2">
        Buy Pro
      </Text>
    </View>
  );
}

mount(App);
