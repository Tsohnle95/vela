export default {
 name:'Passing shade',family:'Light',description:'A soft shadow rolls across each cube beneath the nearby pearl.',rule:'A dark pool follows the pearl’s projection instead of adding another highlight.',
 setup({add,defs,prefix,faceEffects,set}){const g=add(defs,'radialGradient',{id:prefix+'-shade'});add(g,'stop',{offset:0,'stop-color':'#100a40','stop-opacity':.7});add(g,'stop',{offset:1,'stop-color':'#100a40','stop-opacity':0});const nodes=faceEffects.map(fx=>add(fx,'ellipse',{rx:145,ry:180,fill:`url(#${prefix}-shade)`}));return ({i,x,y,dx,dy,strength})=>set(nodes[i],{cx:x+dx*.45,cy:y+dy*.4,opacity:strength*.4});}
};
