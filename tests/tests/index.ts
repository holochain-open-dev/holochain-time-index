import { Scenario, runScenario, getZomeCaller } from '@holochain/tryorama'
import path from 'path'
import test from "tape";

const appBundle = { path: path.join("../workdir/time-index-test.happ") };
const now = new Date("August 12, 2021 14:01:30")

test("test get empty path", async (t) => {
  const scenario = new Scenario();
  try {
    const alice = await scenario.addPlayerWithApp(appBundle);
    const aliceCallZome = getZomeCaller(alice.cells[0], "test_zome");

    var dateOffset = (24*60*60*1000); //1 day ago
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);

    let results_between = await aliceCallZome(
      "get_links_for_time_span", 
      {index: "test_index", from: new Date().toISOString(), until: date.toISOString(), limit: 10}
    );

    //@ts-ignore
    t.equal(results_between.length, 0);
  } catch (error) {
    console.error("error", error);
  }
  await scenario.cleanUp();
});

test("test get index dfs", async (t) => {
  const scenario = new Scenario();
  try {
    const alice = await scenario.addPlayerWithApp(appBundle);

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
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry", 
      payload: {title: "A test index", created: new Date().toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry", 
      payload: {title: "A test index2", created: yesterday.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry", 
      payload: {title: "A test index3", created: twoDaysAgo.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry", 
      payload: {title: "A test index4", created: threeDaysAgo.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry", 
      payload: {title: "A test index5", created: twoMonthsAgo.toISOString()}
    })

    var dateOffset = (24*60*60*1000); //1 day ago
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);

    //Get results in descending order
    let results_between = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_links_for_time_span", 
      payload: {index: "test_index", from: new Date().toISOString(), until: twoMonthsAgo.toISOString(), limit: 10}
    })
    console.log("Got results", results_between);
    //@ts-ignore
    t.equal(results_between.length, 5)
    //@ts-ignore
    for (let i=0; i < results_between.length; i++) {
      if (i != 0) {
        //@ts-ignore
        t.assert(results_between[i].timestamp < results_between[i-1].timestamp)
      }
    }

    //Get results in ascending order
    let asc_results = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_links_for_time_span", 
      payload: {index: "test_index", from: twoMonthsAgo.toISOString(), until: new Date().toISOString(), limit: 10}
    })
    console.log("Got results", asc_results);
    //@ts-ignore
    t.equal(asc_results.length, 5)
  } catch (error) {
    console.error("error", error);
  }
  await scenario.cleanUp();
})

test("test get links and load dfs", async (t) => {
  const scenario = new Scenario();
  try {
    const alice = await scenario.addPlayerWithApp(appBundle);

    var dateOffset = (24*60*60*1000); //1 day ago
    var yesterday = new Date(now.getTime() - dateOffset);

    var dateOffset = (24*60*60*1000) * 2; //2 day ago
    var twoDaysAgo = new Date(now.getTime() - dateOffset);

    var dateOffset = (24*60*60*1000) * 3; //3 day ago
    var threeDaysAgo = new Date(now.getTime() - dateOffset);

    var dateOffset = (24*60*60*1000) * 4; //4 day ago
    var fourDaysAgo = new Date(now.getTime() - dateOffset);

    var dateOffset = (24*60*60*1000) * 60; //2 months ago
    var twoMonthsAgo = new Date(now.getTime() - dateOffset);

    //Index entry
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index", created: now.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index2", created: yesterday.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index3", created: twoDaysAgo.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index4", created: threeDaysAgo.toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index5", created: twoMonthsAgo.toISOString()}
    })

    //Get results in descending order
    let results_between = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_links_and_load_for_time_span",
      payload: {index: "test_index", from: now.toISOString(), until: twoMonthsAgo.toISOString(), limit: 10}
    })
    console.log("Got results", results_between);
    //@ts-ignore
    t.equal(results_between.length, 5)
    //@ts-ignore
    t.equal(results_between[0].title, "A test index")
    //@ts-ignore
    t.equal(results_between[1].title, "A test index2")
    //@ts-ignore
    t.equal(results_between[2].title, "A test index3")
    //@ts-ignore
    t.equal(results_between[3].title, "A test index4")
    //@ts-ignore
    t.equal(results_between[4].title, "A test index5")

    //Get results in ascending order
    let asc_results = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_links_and_load_for_time_span",
      payload: {index: "test_index", from: twoMonthsAgo.toISOString(), until: now.toISOString(), limit: 10}
    })
    console.log("Got results", asc_results);
    //@ts-ignore
    t.equal(asc_results.length, 5)
    //@ts-ignore
    t.equal(asc_results[0].title, "A test index5")
    //@ts-ignore
    t.equal(asc_results[1].title, "A test index4")
    //@ts-ignore
    t.equal(asc_results[2].title, "A test index3")
    //@ts-ignore
    t.equal(asc_results[3].title, "A test index2")
    //@ts-ignore
    t.equal(asc_results[4].title, "A test index")
  } catch (error) {
    console.error("error", error);
  }
  await scenario.cleanUp();
})

test("test simple index", async (t) => {
  const scenario = new Scenario();
  try {
    const alice = await scenario.addPlayerWithApp(appBundle);

    var dateOffset = 10; //10ms
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);

    //Index entry
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index", created: new Date().toISOString()}
    })
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index2", created: date.toISOString()}
    })

    //Create another index for one day ago
    var dateOffset = (24*60*60*1000); //1 day ago
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);
    await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "index_entry",
      payload: {title: "A test index3", created: date.toISOString()}
    })

    let results_between = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_indexes_for_time_span",
      payload: {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10}
    })
    console.log("Got results between", results_between);
    //@ts-ignore
    t.deepEqual(results_between.length, 2);

    //Create another index for one day ago
    var dateOffset = (24*60*60*1000) / 2; //12 hr ago
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);

    let results_betwee2 = await alice.cells[0].callZome({
      zome_name: "test_zome", 
      fn_name: "get_indexes_for_time_span",
      payload: {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10}
    })
    console.log("Got results between", results_betwee2);
    //@ts-ignore
    t.deepEqual(results_betwee2.length, 1);
  } catch (error) {
    console.error("error", error);
  }
  await scenario.cleanUp();
})

test("test delete", async (t) => {
  const scenario = new Scenario();
  try {
    const alice = await scenario.addPlayerWithApp(appBundle);
  
    //Index entry
    await alice.cells[0].callZome({
      zome_name: "test_zome",
      fn_name:  "index_entry",
      payload: {title: "A test index", created: new Date().toISOString()}
    })
  
    //Create another index for one day ago
    var dateOffset = (24*60*60*1000); //1 day ago
    var date = new Date();
    date.setTime(date.getTime() - dateOffset);
  
    let rb = await alice.cells[0].callZome({
      zome_name: "test_zome",
      fn_name:  "get_indexes_for_time_span",
      payload: {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10}
    })
    console.log("Got results between", rb);
    //@ts-ignore
    t.deepEqual(rb.length, 1);
    //@ts-ignore
    console.log("deleting entry at", rb[0].links[0].target);
    
    await alice.cells[0].callZome({
      zome_name: "test_zome",
      fn_name:  "remove_index",
      //@ts-ignore
      payload: rb[0].links[0].target
    })
  
    let rb_pd = await alice.cells[0].callZome({
      zome_name: "test_zome",
      fn_name:  "get_links_for_time_span",
      payload: {index: "test_index", from: date.toISOString(), until: new Date().toISOString(), limit: 10}
    })
    console.log("Got results between post delete", rb_pd);
    //@ts-ignore
    t.deepEqual(rb_pd.length, 0);
  } catch (error) {
    console.error("error", error);
  }
  await scenario.cleanUp();
})
