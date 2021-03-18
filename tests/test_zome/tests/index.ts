import { Orchestrator, Config, InstallAgentsHapps } from '@holochain/tryorama'
import { TransportConfigType, ProxyAcceptConfig, ProxyConfigType } from '@holochain/tryorama'
import { HoloHash, InstallAppRequest } from '@holochain/conductor-api'
import path from 'path'
import * as msgpack from '@msgpack/msgpack';

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

orchestrator.registerScenario("test simple chunk fn's", async (s, t) => {
  // Declare two players using the previously specified config, nicknaming them "alice" and "bob"
  // note that the first argument to players is just an array conductor configs that that will
  // be used to spin up the conductor processes which are returned in a matching array.
  const [alice, bob] = await s.players([conductorConfig, conductorConfig])

  console.log("Init alice happ");
  const req: InstallAppRequest = {
    installed_app_id: `my_app:1234`, // my_app with some unique installed id value
    agent_key: await alice.adminWs().generateAgentPubKey(),
    dnas: [{
      path: path.join(__dirname, '../time-index.dna.gz'),
      nick: `my_cell_nick`,
      properties: {
        "enforce_spam_limit": 20,
        "max_chunk_interval": 100000
      },
      //membrane_proof: Array.from(msgpack.encode({role:"steward", signature:"..."})),
    }]
  }
  const alice_happ = await alice._installHapp(req)
  console.log("Agents init'd\n");

  //Index entry
  let index = await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index", created: new Date().toISOString()})

  var dateOffset = 10000; //10 seconds
  var date = new Date();
  date.setTime(date.getTime() - dateOffset);
  await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index2", created: date.toISOString()})
  
  let get_index = await alice_happ.cells[0].call("time_index", "get_most_recent_indexes", {index: "test_index"})
  console.log("Got index", get_index);
  t.deepEqual(get_index.links.length, 2);

  let get_index_current = await alice_happ.cells[0].call("time_index", "get_current_addresses", {index: "test_index"})
  console.log("Got index", get_index_current);
  t.deepEqual(get_index.links.length, 2);

  //Create another index for one day ago
  var dateOffset = (24*60*60*1000); //1 day ago
  var date = new Date();
  date.setTime(date.getTime() - dateOffset);
  await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index3", created: date.toISOString()})

  let results_between = await alice_happ.cells[0].call("time_index", "get_addresses_between", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
  console.log("Got results between", results_between);
  t.deepEqual(results_between.length, 2);

  //Create another index for one day ago
  var dateOffset = (24*60*60*1000) / 2; //12 hr ago
  var date = new Date();
  date.setTime(date.getTime() - dateOffset);

  let results_betwee2 = await alice_happ.cells[0].call("time_index", "get_addresses_between", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
  console.log("Got results between", results_betwee2);
  t.deepEqual(results_betwee2.length, 1);
})

// Run all registered scenarios as a final step, and gather the report,
// if you set up a reporter
const report = orchestrator.run()

// Note: by default, there will be no report
console.log(report)