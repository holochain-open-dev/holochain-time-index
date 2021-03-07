import { Orchestrator, Config, InstallAgentsHapps } from '@holochain/tryorama'
import { TransportConfigType, ProxyAcceptConfig, ProxyConfigType } from '@holochain/tryorama'
import { HoloHash } from '@holochain/conductor-api'
import path from 'path'

// Set up a Conductor configuration using the handy `Conductor.config` helper.
// Read the docs for more on configuration.
const network = {
    transport_pool: [{
      type: TransportConfigType.Proxy,
      sub_transport: {type: TransportConfigType.Quic},
      proxy_config: {
        type: ProxyConfigType.LocalProxyServer,
        proxy_accept_config: ProxyAcceptConfig.AcceptAll
      }
    }],
    bootstrap_service: "https://bootstrap.holo.host"
};
const conductorConfig = Config.gen();

// create an InstallAgentsHapps array with your DNAs to tell tryorama what
// to install into the conductor.
const installation: InstallAgentsHapps = [
  // agent 0
  [
    // happ 0
    [path.join("../time-chunking.dna.gz"),]
  ]
]

// Instatiate your test's orchestrator.
// It comes loaded with a lot default behavior which can be overridden, including:
// * custom conductor startup
// * custom test result reporting
// * scenario middleware, including integration with other test harnesses
const orchestrator = new Orchestrator()

//const sleep = (ms) => new Promise((resolve) => setTimeout(() => resolve(), ms));

orchestrator.registerScenario("test simple chunk fn's", async (s, t) => {
  // Declare two players using the previously specified config, nicknaming them "alice" and "bob"
  // note that the first argument to players is just an array conductor configs that that will
  // be used to spin up the conductor processes which are returned in a matching array.
  const [alice, bob] = await s.players([conductorConfig, conductorConfig])

  console.log("Init alice happ");
  // install your happs into the conductors and destructuring the returned happ data using the same
  // array structure as you created in your installation array.
  const [
    [alice_happ],
  ] = await alice.installAgentsHapps(installation)
  // const [
  //   [bob_sc_happ],
  // ] = await bob.installAgentsHapps(installation)
  console.log("Agents init'd\n");

  //Get genesis chunk
  let genesis = await alice_happ.cells[0].call("time_chunking", "get_genesis_chunk", null)
  console.log("Got genesis chunk", genesis);
  t.notEqual(genesis.from, undefined)

  let current_chunk = await alice_happ.cells[0].call("time_chunking", "get_current_chunk", null);
  t.deepEqual(genesis, current_chunk);

  let search_current_chunk = await alice_happ.cells[0].call("time_chunking", "get_latest_chunk", null);
  t.deepEqual(genesis, search_current_chunk);

  let chunk_size = await alice_happ.cells[0].call("time_chunking", "get_max_chunk_interval", null);
  t.notEqual(chunk_size, undefined);

  //Create a fake next chunk to see if we can go back and get the genesis
  let next_chunk = {
    from: {secs: genesis.from.secs + chunk_size.secs, nanos: genesis.from.nanos + chunk_size.nanos },
    until: {secs: genesis.until.secs + chunk_size.secs, nanos: genesis.until.nanos + chunk_size.nanos },
  };
  let get_previous_chunk = await alice_happ.cells[0].call("time_chunking", "get_previous_chunk", {chunk: next_chunk, hops: 1})
  console.log("Got previous chunk", get_previous_chunk);
  t.deepEqual(genesis, get_previous_chunk);
})

// Run all registered scenarios as a final step, and gather the report,
// if you set up a reporter
const report = orchestrator.run()

// Note: by default, there will be no report
console.log(report)