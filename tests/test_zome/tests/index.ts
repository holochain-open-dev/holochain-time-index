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
    [path.join("../time-index.dna.gz"),]
  ]
]

// Instatiate your test's orchestrator.
// It comes loaded with a lot default behavior which can be overridden, including:
// * custom conductor startup
// * custom test result reporting
// * scenario middleware, including integration with other test harnesses
const orchestrator = new Orchestrator()

const sleep = (ms) => new Promise((resolve) => setTimeout(() => resolve(), ms));

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
  sleep(10000);

  //Index entry
  let index = await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index", created: new Date().toISOString()})
  
  let get_index = await alice_happ.cells[0].call("time_index", "get_most_recent_indexes", {index: "test_index"})
  console.log("Get index", get_index);
})

// Run all registered scenarios as a final step, and gather the report,
// if you set up a reporter
const report = orchestrator.run()

// Note: by default, there will be no report
console.log(report)