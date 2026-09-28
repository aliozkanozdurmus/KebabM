import { serveStdio } from '@modelcontextprotocol/server/stdio';
import { createServer } from '../server.mjs';
serveStdio(()=>createServer(async(tool,args,{signal}={})=>{
  if(tool==='store_secret')return {stored:true,provider:args.provider};
  if(tool==='search_knowledge')throw new Error('provider rejected key=AIzaFakeSecretExample');
  if(tool==='embed_project')await new Promise((resolve,reject)=>{const t=setTimeout(resolve,100);signal?.addEventListener('abort',()=>{clearTimeout(t);reject(new Error('Cancelled'));},{once:true});});
  return {tool,args};
}));
