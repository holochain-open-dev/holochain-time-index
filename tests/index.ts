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
const conductorConfig = Config.gen({network});
//const conductorConfig = Config.gen();

// Construct proper paths for your DNAs
const chunking = path.join(__dirname, '../time-chunking.dna.gz')

// create an InstallAgentsHapps array with your DNAs to tell tryorama what
// to install into the conductor.
const installation: InstallAgentsHapps = [
  // agent 0
  [
    // happ 0
    [chunking]
  ]
]

// Instatiate your test's orchestrator.
// It comes loaded with a lot default behavior which can be overridden, including:
// * custom conductor startup
// * custom test result reporting
// * scenario middleware, including integration with other test harnesses
const orchestrator = new Orchestrator()

const sleep = (ms) => new Promise((resolve) => setTimeout(() => resolve(), ms));

orchestrator.registerScenario("create and get simple chunked link", async (s, t) => {
  // Declare two players using the previously specified config, nicknaming them "alice" and "bob"
  // note that the first argument to players is just an array conductor configs that that will
  // be used to spin up the conductor processes which are returned in a matching array.
  const [alice, bob] = await s.players([conductorConfig, conductorConfig])

  // install your happs into the conductors and destructuring the returned happ data using the same
  // array structure as you created in your installation array.
  const [
    [alice_happ],
  ] = await alice.installAgentsHapps(installation)
  const [
    [bob_sc_happ],
  ] = await bob.installAgentsHapps(installation)

  //Create communication
  let source = await alice_happ.cells[0].call("time_chunking", "get_genesis_chunk", {})
  console.log("Got source", source);
})

// Run all registered scenarios as a final step, and gather the report,
// if you set up a reporter
const report = orchestrator.run()

// Note: by default, there will be no report
console.log(report)