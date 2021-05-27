# PINT launch

Launch PINT with polkadot and statemint

```typescript
import { local, docker } from "@pint/launch";
import { StdioOptions } from "child_process";

async main() {
    const stdio = "inherit";
    const localPs: ChildProcess = await local(stdio);
    const dockerPs: ChildProcess = await local(stdio);
}

```

## LICENSE

GNU-v3
