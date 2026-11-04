export default {
  name:'Proximity sheen',family:'Light',description:'A soft patch of light follows the pearl across the nearest cube.',rule:'The highlight tracks the pearl’s projected position and fades with distance.',
  setup({add,defs,prefix,faceEffects,set}){
    const grad=add(defs,'radialGradient',{id:prefix+'-pool'});add(grad,'stop',{offset:0,'stop-color':'#fff','stop-opacity':.65});add(grad,'stop',{offset:1,'stop-color':'#fff','stop-opacity':0});
    const pools=faceEffects.map(g=>add(g,'ellipse',{rx:145,ry:120,fill:`url(#${prefix}-pool)`}));
    return ({i,x,y,dx,dy,strength})=>set(pools[i],{cx:x+dx*.38,cy:y+dy*.38,opacity:strength*.35});
  }
};
