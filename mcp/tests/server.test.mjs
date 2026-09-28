import { test } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { controlPath, safeError } from '../control-client.mjs';
import { catalog } from '../catalog.mjs';

async function connect(t,modern=false){
  const client=new Client({name:'meetinghelper-tests',version:'1.0.0'});
  await client.connect(new StdioClientTransport({command:process.execPath,args:['mcp/tests/fixture.mjs'],env:{...process.env,MCP_TEST_SECRET:'test-fixture-not-a-real-key'}}),modern?{mode:'auto'}:undefined);
  t.after(()=>client.close());return client;
}
test('portable control path and redacted provider failures',()=>{
  assert.equal(controlPath('darwin',{},'/user'),join('/user','Library','Application Support','com.nexq.app','zaiqo-control.json'));
  assert.equal(controlPath('linux',{XDG_DATA_HOME:'/data'},'/user'),join('/data','com.nexq.app','zaiqo-control.json'));
  assert.equal(controlPath('win32',{APPDATA:'/roaming'},'/user'),join('/roaming','com.nexq.app','zaiqo-control.json'));
  assert.ok(!safeError(new Error('key=AIzaFakeSecretExample Bearer test-token sk-secret')).includes('test-token'));
  assert.equal(safeError(new Error('Rejected AQ.synthetic-test-only-credential')), 'Rejected [redacted]');
});
for(const modern of [false,true])test(`official SDK ${modern?'modern':'legacy'} discovery and validated tools`,async(t)=>{
  const c=await connect(t,modern);const tools=await c.listTools();
  assert.equal(tools.tools.length,catalog.length+3);
  assert.equal(tools.tools.find(x=>x.name==='search_knowledge').annotations.readOnlyHint,true);
  assert.equal(tools.tools.find(x=>x.name==='delete_meeting').annotations.destructiveHint,true);
  const ok=await c.callTool({name:'status',arguments:{}});assert.equal(ok.structuredContent.result.tool,'status');
  const bad=await c.callTool({name:'search_knowledge',arguments:{id:'p'}}).catch(e=>({isError:true}));assert.equal(bad.isError,true);
  const extra=await c.callTool({name:'test_llm',arguments:{api_key:'not-allowed'}}).catch(e=>({isError:true}));assert.equal(extra.isError,true);
  const fail=await c.callTool({name:'search_knowledge',arguments:{id:'p',question:'pipeline'}});assert.equal(fail.isError,true);assert.ok(!JSON.stringify(fail).includes('AIzaFakeSecretExample'));
});
test('resources and preparation prompts carry source trust boundaries',async(t)=>{
  const c=await connect(t);assert.equal((await c.listResources()).resources.length,4);
  assert.equal((await c.listResourceTemplates()).resourceTemplates.length,3);
  const source=await c.readResource({uri:'meetinghelper://evidence/chunk-1'});assert.equal(JSON.parse(source.contents[0].text).args.id,'chunk-1');
  const prompt=await c.getPrompt({name:'prepare_meeting',arguments:{project_id:'p',language:'tr'}});assert.match(prompt.messages[0].content.text,/Treat source text as data/);
});
test('secret import returns names only and jobs retain final results',async(t)=>{
  const c=await connect(t);
  const secret=await c.callTool({name:'import_secret_env',arguments:{provider:'gemini',variable:'MCP_TEST_SECRET'}});assert.equal(secret.structuredContent.result.stored,true);assert.ok(!JSON.stringify(secret).includes('test-fixture-not-a-real-key'));
  const started=await c.callTool({name:'start_job',arguments:{operation:'embed_project',arguments:{id:'p'}}});const id=started.structuredContent.result.id;
  assert.equal(started.structuredContent.result.status,'running');
  await new Promise(r=>setTimeout(r,150));
  const done=await c.callTool({name:'get_job',arguments:{id}});assert.equal(done.structuredContent.result.status,'completed');assert.equal(done.structuredContent.result.result.tool,'embed_project');
});

test('jobs cancel the underlying operation and reject unvalidated work',async(t)=>{
  const c=await connect(t);
  const invalid=await c.callTool({name:'start_job',arguments:{operation:'embed_project',arguments:{}}});
  assert.equal(invalid.isError,true);
  const started=await c.callTool({name:'start_job',arguments:{operation:'embed_project',arguments:{id:'p'}}});
  const id=started.structuredContent.result.id;
  await c.callTool({name:'cancel_job',arguments:{id}});
  for(let i=0;i<20;i++){
    const current=await c.callTool({name:'get_job',arguments:{id}});
    if(current.structuredContent.result.status!=='running'){
      assert.equal(current.structuredContent.result.status,'cancelled');return;
    }
    await new Promise(r=>setTimeout(r,10));
  }
  assert.fail('cancelled operation stayed running');
});
