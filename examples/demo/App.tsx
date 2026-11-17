import { mount, signal, Scroll, Text, View, type JSX } from "nsh";
import {
  Purchases,
  PurchasesError,
  useEntitlements,
  type Entitlement,
  type EntitlementsState,
  type Product,
  type ProductKind,
  type Purchase,
  type PurchaseErrorCode,
  type PurchaseState,
} from "../../packages/nsh-purchases/src";
import { memoryBackend } from "../../packages/nsh-purchases/src/memory";

/** The fake store every action below runs on: no real store, no real money. */
const backend = memoryBackend();
const purchases = new Purchases({ backend });

const FIXTURE_IDS = ["pro_unlock", "coins_100", "pro_monthly"];

const KIND_LABELS: Record<ProductKind, string> = {
  consumable: "Consumable",
  nonConsumable: "Non-consumable",
  subscription: "Subscription",
};

const storeState = signal("connecting");
const ready = signal(false);
const products = signal<Product[]>([]);
const pendingList = signal<Purchase[]>([]);
/** The product whose fake store sheet stands open; empty when no purchase runs. */
const sheetFor = signal("");
const message = signal("nothing yet");
const detail = signal("");

function codeOf(error: unknown): PurchaseErrorCode | "unknown error" {
  return error instanceof PurchasesError ? error.code : "unknown error";
}

void (async () => {
  try {
    if (!(await purchases.isAvailable())) {
      storeState.set("the store is not available");
      return;
    }
    await purchases.initialize();
    products.set(await purchases.products(FIXTURE_IDS));
    storeState.set(`initialized · storefront ${await purchases.countryCode()}`);
    ready.set(true);
  } catch (error: unknown) {
    storeState.set(`failed ${codeOf(error)}`);
  }
})();

const kindOf = (productId: string): ProductKind | undefined =>
  products().find((product) => product.id === productId)?.kind;

function describeRight(right: Entitlement): string {
  if (right.expiresAtMs === undefined) return `${right.productId} · active`;
  return `${right.productId} · active · expires ${new Date(right.expiresAtMs).toISOString().slice(0, 10)}`;
}

function describeRights(state: EntitlementsState): string {
  if (state.status === "checking") return "checking";
  if (state.status === "failed") return state.error.code;
  return "what the user owns right now";
}

/** A separate helper narrows the state once; a second `state()` call in JSX would not. */
function rightsRows(state: EntitlementsState): JSX.Element | null {
  if (state.status !== "ready") return null;
  return (
    <Each of={state.entitlements}>
      {(right) => (
        <Text testID={`right-${right().productId}`} class="font-mono text-sm text-foreground">
          {describeRight(right())}
        </Text>
      )}
    </Each>
  );
}

function ActionButton(props: { testID: string; label: string; onPress: () => void; class: string }) {
  return (
    <Text testID={props.testID} role="button" onPress={props.onPress} class={props.class}>
      {props.label}
    </Text>
  );
}

function Store() {
  const entitlements = useEntitlements({ purchases });
  const answerClass = "rounded-lg bg-muted px-3 py-2 text-sm text-foreground";

  const refresh = async () => {
    pendingList.set(await purchases.pending());
    entitlements.refresh();
  };

  const failed = (error: unknown) => {
    sheetFor.set("");
    const code = codeOf(error);
    message.set(code);
    detail.set(error instanceof PurchasesError ? error.message : "");
  };

  const buy = (product: Product) => {
    sheetFor.set(product.id);
    detail.set("");
    message.set(`buying ${product.id}`);
    purchases.purchase(product.id, {}).then(
      (purchase) => {
        sheetFor.set("");
        message.set(`bought ${purchase.productId} (${purchase.state})`);
        void refresh();
      },
      failed,
    );
  };

  // The sheet's answer, as the fake store would give it: a state, or a code to reject with.
  const answer = (asStore: PurchaseState | PurchaseErrorCode) => {
    backend.finishPurchase(asStore);
  };

  const finish = (purchase: Purchase) => {
    purchases.finish(purchase).then(
      () => {
        message.set(`finished ${purchase.id}`);
        detail.set("");
        void refresh();
      },
      failed,
    );
  };

  const consume = (purchase: Purchase) => {
    purchases.consume(purchase).then(
      () => {
        message.set(`consumed ${purchase.id}`);
        detail.set("");
        void refresh();
      },
      failed,
    );
  };

  const restore = () => {
    purchases.restore().then(
      (restored) => {
        const ids = restored.map((purchase) => purchase.productId).join(", ");
        message.set(restored.length === 0 ? "restore: nothing to restore" : `restored ${ids}`);
        detail.set("");
        void refresh();
      },
      failed,
    );
  };

  return (
    <Scroll class="flex-1">
      <View class="gap-4 p-4">
        <View class="gap-1">
          <Text testID="title" class="text-lg font-semibold text-foreground">
            nsh-purchases demo
          </Text>
          <Text class="text-xs text-muted-foreground">
            Running on the in-memory fake store from nsh-purchases/memory — no real store, no real money.
          </Text>
          <Text testID="store" class="font-mono text-xs text-muted-foreground">
            {storeState()}
          </Text>
        </View>

        <View testID="last-result" class="gap-1">
          <Text testID="message" class="font-mono text-sm text-foreground">
            {message()}
          </Text>
          {detail() !== "" && (
            <Text testID="detail" class="font-mono text-xs text-muted-foreground">
              {detail()}
            </Text>
          )}
        </View>

        {sheetFor() !== "" && (
          <View testID="sheet" class="gap-2 rounded-lg border border-border bg-card p-3">
            <Text class="text-xs text-muted-foreground">
              The fake store is asking about {sheetFor()}. Answer it as a store would.
            </Text>
            <View class="flex-row flex-wrap gap-2">
              <ActionButton testID="answer-purchased" label="Approve" onPress={() => answer("purchased")} class={answerClass} />
              <ActionButton testID="answer-pending" label="Approve as pending" onPress={() => answer("pending")} class={answerClass} />
              <ActionButton testID="answer-cancelled" label="Cancel" onPress={() => answer("ERR_PURCHASES_CANCELLED")} class={answerClass} />
              <ActionButton testID="answer-payment" label="Decline" onPress={() => answer("ERR_PURCHASES_PAYMENT_INVALID")} class={answerClass} />
              <ActionButton testID="answer-owned" label="Already owned" onPress={() => answer("ERR_PURCHASES_ALREADY_OWNED")} class={answerClass} />
              <ActionButton testID="app-cancel" label="Cancel from the app" onPress={() => purchases.cancel()} class={answerClass} />
            </View>
          </View>
        )}

        <View class="gap-2">
          <Text class="text-sm font-semibold text-muted-foreground">Products</Text>
          <Each of={products()}>
            {(product) => (
              <View
                testID={`product-${product().id}`}
                class="flex-row items-center justify-between gap-2 rounded-lg border border-border bg-card p-3"
              >
                <View class="min-w-0 flex-1 gap-0.5">
                  <Text class="text-sm font-medium text-foreground">{product().title}</Text>
                  <Text class="font-mono text-xs text-muted-foreground">
                    {`${product().id} · ${product().price} ${product().currencyCode ?? ""} · ${KIND_LABELS[product().kind]}`}
                  </Text>
                </View>
                <ActionButton
                  testID={`buy-${product().id}`}
                  label="Buy"
                  onPress={() => buy(product())}
                  class="rounded-lg bg-primary px-3 py-2 text-sm text-primary-foreground"
                />
              </View>
            )}
          </Each>
        </View>

        <View class="gap-2">
          <Text class="text-sm font-semibold text-muted-foreground">Pending purchases</Text>
          {pendingList().length === 0 ? (
            <Text testID="no-pending" class="text-xs text-muted-foreground">
              nothing pending
            </Text>
          ) : (
            <Each of={pendingList()}>
              {(purchase) => (
                <View
                  testID={`pending-${purchase().id}`}
                  class="flex-row items-center justify-between gap-2 rounded-lg border border-border bg-card p-3"
                >
                  <View class="min-w-0 flex-1 gap-0.5">
                    <Text class="text-sm font-medium text-foreground">{purchase().productId}</Text>
                    <Text class="font-mono text-xs text-muted-foreground">{`${purchase().id} · ${purchase().state}`}</Text>
                  </View>
                  {kindOf(purchase().productId) === "consumable" ? (
                    <ActionButton testID={`consume-${purchase().id}`} label="Consume" onPress={() => consume(purchase())} class={answerClass} />
                  ) : (
                    <ActionButton testID={`finish-${purchase().id}`} label="Finish" onPress={() => finish(purchase())} class={answerClass} />
                  )}
                </View>
              )}
            </Each>
          )}
        </View>

        <View class="gap-2">
          <Text class="text-sm font-semibold text-muted-foreground">Entitlements</Text>
          <Text testID="rights" class="font-mono text-xs text-muted-foreground">
            {describeRights(entitlements.state())}
          </Text>
          {rightsRows(entitlements.state())}
        </View>

        <ActionButton testID="restore" label="Restore purchases" onPress={restore} class={answerClass} />
      </View>
    </Scroll>
  );
}

function App() {
  // A component's body runs once; `If` is what answers a signal changing later.
  return (
    <If
      when={ready()}
      else={
        <View class="flex-1 items-center justify-center gap-1 p-4">
          <Text testID="title" class="text-lg font-semibold text-foreground">
            nsh-purchases demo
          </Text>
          <Text testID="connecting" class="text-xs text-muted-foreground">
            {`Connecting to the in-memory fake store (${storeState()})`}
          </Text>
        </View>
      }
    >
      {() => <Store />}
    </If>
  );
}

mount(App);
