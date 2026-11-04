import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {createServer} from 'vite';
import vue from '@vitejs/plugin-vue';
const require=createRequire(import.meta.url),{chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const html=`<!doctype html><html><head><style>*{box-sizing:border-box}html,body,#app{margin:0;height:100%;font-family:Arial,sans-serif}button,select{cursor:pointer}button,select,input{font:inherit}button{padding:0}</style></head><body><div id="app"></div><script type="module">
import {createApp,h,ref} from '/node_modules/.vite/deps/vue.js';
import Galaxy from '/src/view/commit/CommitGalaxyPanel.vue';
const branches=['main','feature/api','feature/ui','release'].map(name=>({name,is_head:name==='main',is_remote:false,upstream:null,ahead:0,behind:0}));
const commits=Array.from({length:900},(_,i)=>({sha:'sha'+i,short_sha:('000000'+i).slice(-7),message:'Commit '+i,author_name:'Test',author_email:'test@example.invalid',timestamp:i,time_ago:'today',parent_shas:i?[i%90===0?'sha'+(i-5):'sha'+(i-1),...(i%90===0?['sha'+(i-1)]:[])]:[],refs:i===899?['main']:i===890?['feature/api']:i===880?['feature/ui']:i===870?['release']:[]}));
const visible=ref(true),selected=ref(null);window.selected=null;window.toggle=()=>visible.value=!visible.value;
createApp({render:()=>visible.value?h(Galaxy,{commits,branches,currentBranch:'main',selectedSha:selected.value,onSelect:p=>{window.selected=p.commit.sha;selected.value=p.commit.sha},onClose:()=>visible.value=false}):h('div','Closed')}).mount('#app');
</script></body></html>`;
const server=await createServer({root,configFile:false,plugins:[{name:'galaxy-test',configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url!=='/__galaxy_test__')return next();void server.transformIndexHtml(req.url,html).then(result=>{res.setHeader('Content-Type','text/html');res.end(result);});});}},vue()],resolve:{alias:{'@':path.join(root,'src')}},optimizeDeps:{entries:[],noDiscovery:true,include:['vue','lucide-vue-next','three','three/addons/controls/OrbitControls.js']},server:{port:0,host:'127.0.0.1'}});
let browser;
try{
 await server.listen();browser=await chromium.launch({headless:true,args:['--enable-unsafe-swiftshader'],...(process.env.CHROME_PATH?{executablePath:process.env.CHROME_PATH}:{})});
 const page=await browser.newPage({viewport:{width:1280,height:860}}),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.goto('http://127.0.0.1:'+server.httpServer.address().port+'/__galaxy_test__',{waitUntil:'domcontentloaded',timeout:60000});
 await page.waitForFunction(()=>document.querySelectorAll('.galaxy-label').length>0,{},{timeout:60000});
 const canvas=page.locator('.galaxy-canvas');
 const shot=await canvas.screenshot({path:path.join(root,'src-tauri/target/galaxy-desktop.png')});
 const pixels=await page.evaluate(async data=>{
  const image=new Image();image.src='data:image/png;base64,'+data;await image.decode();const c=document.createElement('canvas');c.width=image.width;c.height=image.height;const x=c.getContext('2d');x.drawImage(image,0,0);const bytes=x.getImageData(0,0,c.width,c.height).data;let bright=0;for(let i=0;i<bytes.length;i+=4)if(Math.max(bytes[i],bytes[i+1],bytes[i+2])>40)bright++;return {bright,total:c.width*c.height};
 },shot.toString('base64'));
 assert.ok(pixels.bright>1000,'galaxy must contain rendered stars, not a blank canvas');
 const label=await page.locator('.galaxy-label').first().boundingBox();
 await page.mouse.click(label.x-9,label.y+label.height/2);
 await page.waitForFunction(()=>window.selected!==null);
 await page.mouse.move(0,0);
 const before=await canvas.screenshot();await page.getByLabel('Galaxy motion',{exact:true}).selectOption('circling');await page.waitForTimeout(600);
 const after=await canvas.screenshot();assert.ok(!before.equals(after),'circling must move the scene');
 await page.emulateMedia({reducedMotion:'reduce'});await page.waitForTimeout(500);const paused=await canvas.screenshot();await page.waitForTimeout(300);assert.ok(paused.equals(await canvas.screenshot()),'reduced motion must stop circling');
 const box=await canvas.boundingBox();await page.mouse.move(box.x+box.width/2,box.y+box.height/2);await page.mouse.down();await page.mouse.move(box.x+box.width/2+100,box.y+box.height/2+40,{steps:8});await page.mouse.up();await page.mouse.move(0,0);
 assert.ok(!paused.equals(await canvas.screenshot()),'drag must rotate the view');
 await page.getByLabel('Reset view',{exact:true}).click();await page.getByLabel('Galaxy layout',{exact:true}).selectOption('tree');await page.screenshot({path:path.join(root,'src-tauri/target/galaxy-tree.png')});
 await page.getByLabel('Galaxy layout',{exact:true}).selectOption('spiral');
 await page.setViewportSize({width:420,height:760});await page.waitForTimeout(300);
 assert.ok(await page.locator('.galaxy-panel').evaluate(el=>el.scrollWidth<=el.clientWidth));
 const narrow=await canvas.boundingBox();assert.ok(narrow.height>400);await page.screenshot({path:path.join(root,'src-tauri/target/galaxy-narrow.png')});
 for(let i=0;i<3;i++){await page.getByLabel('Back to Git graph',{exact:true}).click();assert.equal(await canvas.count(),0);await page.evaluate(()=>toggle());await page.waitForFunction(()=>document.querySelectorAll('.galaxy-label').length>0);}
 assert.deepEqual(errors,[]);console.log(JSON.stringify({result:'passed',pixels,checks:'desktop/narrow nonblank scene, circling, reduced motion, drag, tree, reopen/dispose'}));
}finally{await browser?.close();await server.close();}
