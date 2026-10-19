export default {
 name:'Top-face pool',family:'Light',description:'A small pool of light glides over the top facet as the pearl passes.',rule:'The pool is clipped to the top face and follows the pearl’s direction.',
 setup({add,defs,prefix,faceEffects,tops,set}){
 const g=add(defs,'radialGradient',{id:prefix+'-top-pool'});add(g,'stop',{offset:0,'stop-color':'#fff','stop-opacity':.65});add(g,'stop',{offset:1,'stop-color':'#fff','stop-opacity':0});
 const pools=faceEffects.map((fx,i)=>{const clip=add(defs,'clipPath',{id:prefix+'-only-top'+i});add(clip,'path',{d:tops[i]});return add(fx,'ellipse',{rx:120,ry:65,fill:`url(#${prefix}-top-pool)`,'clip-path':`url(#${prefix}-only-top${i})`});});
 return ({i,x,y,dx,dy,strength})=>set(pools[i],{cx:x+dx*.35,cy:y-75+dy*.13,opacity:strength*.55});
 }
};
