import { Orchestrator, Config, InstallAgentsHapps } from '@holochain/tryorama'
import { TransportConfigType, ProxyAcceptConfig, ProxyConfigType } from '@holochain/tryorama'
import { HoloHash, InstallAppRequest } from '@holochain/conductor-api'
import path from 'path'
import * as msgpack from '@msgpack/msgpack';

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

const orchestrator = new Orchestrator()

const installation: InstallAgentsHapps = [
  // agent 0
  [
    // happ 0
    [path.join("../workdir/time-index-test.dna")]
  ]
]


orchestrator.registerScenario("test get index dfs", async (s, t) => {
  const [alice] = await s.players([conductorConfig])
  console.log("Init alice happ");
  const [[alice_happ]] = await alice.installAgentsHapps(installation)

  var dateOffset = (24*60*60*1000); //1 day ago
  var yesterday = new Date();
  yesterday.setTime(yesterday.getTime() - dateOffset);

  var dateOffset = (24*60*60*1000) * 2; //1 day ago
  var twoDaysAgo = new Date();
  twoDaysAgo.setTime(twoDaysAgo.getTime() - dateOffset);

  var dateOffset = (24*60*60*1000) * 3; //1 day ago
  var threeDaysAgo = new Date();
  threeDaysAgo.setTime(threeDaysAgo.getTime() - dateOffset);

  var dateOffset = (24*60*60*1000) * 4; //1 day ago
  var fourDaysAgo = new Date();
  fourDaysAgo.setTime(fourDaysAgo.getTime() - dateOffset);

  var dateOffset = (24*60*60*1000) * 60; //1 day ago
  var twoMonthsAgo = new Date();
  twoMonthsAgo.setTime(twoMonthsAgo.getTime() - dateOffset);

  //Index entry
  await alice_happ.cells[0].call("testing_zome", "index_entry", {title: "A test index", created: new Date().toISOString()})
  await alice_happ.cells[0].call("testing_zome", "index_entry", {title: "A test index2", created: yesterday.toISOString()})
  await alice_happ.cells[0].call("testing_zome", "index_entry", {title: "A test index3", created: twoDaysAgo.toISOString()})
  await alice_happ.cells[0].call("testing_zome", "index_entry", {title: "A test index4", created: threeDaysAgo.toISOString()})
  await alice_happ.cells[0].call("testing_zome", "index_entry", {title: "A test index5", created: twoMonthsAgo.toISOString()})

  var dateOffset = (24*60*60*1000); //1 day ago
  var date = new Date();
  date.setTime(date.getTime() - dateOffset);

  //Get results in descending order
  let results_between = await alice_happ.cells[0].call("testing_zome", "get_links_for_time_span", {index: "test_index", from: new Date().toISOString(), until: twoMonthsAgo.toISOString(), limit: 10})
  console.log("Got results", results_between);
  t.equal(results_between.length, 5)

  //Get results in ascending order
  let asc_results = await alice_happ.cells[0].call("testing_zome", "get_links_for_time_span", {index: "test_index", from: twoMonthsAgo.toISOString(), until: new Date().toISOString(), limit: 10})
  console.log("Got results", asc_results);
  t.equal(asc_results.length, 5)
})

// orchestrator.registerScenario("test simple index", async (s, t) => {
//   const [alice] = await s.players([conductorConfig])

//   console.log("Init alice happ");
//   const req: InstallAppRequest = {
//     installed_app_id: `my_app:1234`, // my_app with some unique installed id value
//     agent_key: await alice.adminWs().generateAgentPubKey(),
//     dnas: [{
//       path: path.join(__dirname, '../time-index.dna.gz'),
//       nick: `my_cell_nick`,
//       properties: {
//         "enforce_spam_limit": 20,
//         "max_chunk_interval": 100000
//       },
//       //membrane_proof: Array.from(msgpack.encode({role:"steward", signature:"..."})),
//     }]
//   }
//   const alice_happ = await alice._installHapp(req)
//   console.log("Agents init'd\n");

//   //Index entry
//   await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index", created: new Date().toISOString()})

//   var dateOffset = 10000; //10 seconds
//   var date = new Date();
//   date.setTime(date.getTime() - dateOffset);
//   await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index2", created: date.toISOString()})
  
//   let get_index = await alice_happ.cells[0].call("time_index", "get_most_recent_indexes", {index: "test_index"})
//   console.log("Got index", get_index);
//   t.deepEqual(get_index.links.length, 2);

//   let get_index_current = await alice_happ.cells[0].call("time_index", "get_current_addresses", {index: "test_index"})
//   console.log("Got index", get_index_current);
//   t.deepEqual(get_index.links.length, 2);

//   //Create another index for one day ago
//   var dateOffset = (24*60*60*1000); //1 day ago
//   var date = new Date();
//   date.setTime(date.getTime() - dateOffset);
//   await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index3", created: date.toISOString()})

//   let results_between = await alice_happ.cells[0].call("time_index", "get_indexes_for_time_span", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
//   console.log("Got results between", results_between);
//   t.deepEqual(results_between.length, 2);

//   //Create another index for one day ago
//   var dateOffset = (24*60*60*1000) / 2; //12 hr ago
//   var date = new Date();
//   date.setTime(date.getTime() - dateOffset);

//   let results_betwee2 = await alice_happ.cells[0].call("time_index", "get_indexes_for_time_span", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
//   console.log("Got results between", results_betwee2);
//   t.deepEqual(results_betwee2.length, 1);
// })

// orchestrator.registerScenario("test delete", async (s, t) => {
//   const [alice] = await s.players([conductorConfig])

//   console.log("Init alice happ");
//   const req: InstallAppRequest = {
//     installed_app_id: `my_app:1234`, // my_app with some unique installed id value
//     agent_key: await alice.adminWs().generateAgentPubKey(),
//     dnas: [{
//       path: path.join(__dirname, '../time-index.dna.gz'),
//       nick: `my_cell_nick`,
//       properties: {
//         "enforce_spam_limit": 20,
//         "max_chunk_interval": 100000
//       },
//       //membrane_proof: Array.from(msgpack.encode({role:"steward", signature:"..."})),
//     }]
//   }
//   const alice_happ = await alice._installHapp(req)
//   console.log("Agents init'd\n");

//   //Index entry
//   await alice_happ.cells[0].call("time_index", "index_entry", {title: "A test index", created: new Date().toISOString()})

//   //Create another index for one day ago
//   var dateOffset = (24*60*60*1000); //1 day ago
//   var date = new Date();
//   date.setTime(date.getTime() - dateOffset);

//   let rb = await alice_happ.cells[0].call("time_index", "get_indexes_for_time_span", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
//   console.log("Got results between", rb);
//   t.deepEqual(rb.length, 1);
//   console.log("deleting entry at", rb[0].links[0].target);
  
//   await alice_happ.cells[0].call("time_index", "remove_index", rb[0].links[0].target)

//   let rb_pd = await alice_happ.cells[0].call("time_index", "get_links_for_time_span", {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10})
//   console.log("Got results between post delete", rb_pd);
//   t.deepEqual(rb_pd.length, 0);
// })

const report = orchestrator.run()
console.log(report)