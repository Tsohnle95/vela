export default {
 name:'Surface ripples',family:'Surface',description:'Fine elliptical ripples contract around the pearl’s reflection, then open as it leaves.',rule:'Ring size follows distance, while their center follows the projected pearl.',
 setup({add,faceEffects,set}){const rings=faceEffects.map(g=>[0,1,2].map(()=>add(g,'ellipse',{fill:'none',stroke:'#eee0ff','stroke-width':2})));return ({i,x,y,dx,dy,distance,strength})=>rings[i].forEach((n,j)=>set(n,{cx:x+dx*.3,cy:y+dy*.22,rx:22+distance*.16+j*22,ry:10+distance*.075+j*10,opacity:strength*(.28-j*.06)}));}
};
