import assert from 'node:assert/strict';
import test from 'node:test';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
const source=readFileSync(new URL('../src/view/commit/galaxyLayout.ts',import.meta.url),'utf8'),module={exports:{}};
new Function('module','exports',ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText)(module,module.exports);
const {buildGalaxyScene,canOrbit,orbitDelta,orderGalaxyCommits}=module.exports;
const commit=(sha,parent_shas=[],timestamp=1,refs=[])=>({sha,short_sha:sha,message:sha,author_name:'Test',author_email:'test@example.invalid',timestamp,time_ago:'now',parent_shas,refs});
test('galaxy layout is deterministic, parents precede children even with clock skew',()=>{
 const commits=[commit('c',['b','feature'],2,['main']),commit('feature',['a'],3,['feature']),commit('a',[],6),commit('b',['a'],1)];
 const branches=['main','feature'].map(name=>({name}));
 const scene=buildGalaxyScene(commits,branches,'main');
 assert.deepEqual(scene,buildGalaxyScene(commits,branches,'main'));
 const positions=new Map(scene.nodes.map((n,i)=>[n.commit.sha,i]));
 for(const [a,b] of scene.edges)assert.ok(a<b);
 assert.ok(positions.get('a')<positions.get('b'));assert.notEqual(scene.nodes[positions.get('feature')].lane,scene.nodes[positions.get('c')].lane);
 assert.ok(scene.nodes.every(n=>Object.values(n.positions).every(p=>p.every(Number.isFinite))));
});
test('iterative ancestry traversal handles large histories and malformed cycles without recursion',()=>{
 const history=Array.from({length:12000},(_,i)=>commit(String(i),i?[String(i-1)]:[],i));
 assert.equal(orderGalaxyCommits(history.reverse()).length,12000);
 assert.equal(orderGalaxyCommits([commit('a',['b']),commit('b',['a'])]).length,2);
});
test('circling pauses for hover, interaction, hidden tabs, tree and reduced motion; time jumps are clamped',()=>{
 const state={motion:'circling',layout:'spiral',hidden:false,reducedMotion:false,interacting:false,hovering:false,resumeAt:0};
 assert.equal(canOrbit(state,100),true);
 for(const key of ['hidden','reducedMotion','interacting','hovering'])assert.equal(canOrbit({...state,[key]:true},100),false);
 assert.equal(canOrbit({...state,layout:'tree'},100),false);assert.equal(canOrbit({...state,resumeAt:200},100),false);
 assert.equal(orbitDelta(null,100),0);assert.equal(orbitDelta(100,100000),0.05);
});
