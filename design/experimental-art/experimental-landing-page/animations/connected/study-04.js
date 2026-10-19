export default {
 name:'Glass reflection',family:'Surface',description:'The nearest cube holds a small, softened reflection of the orbiting pearl.',rule:'The reflection moves with the pearl and disappears when it leaves the cube’s vicinity.',
 setup({circle,faceEffects,set}){const nodes=faceEffects.map(g=>circle(g,14,'#f5edff'));return ({i,x,y,dx,dy,strength})=>set(nodes[i],{cx:x+dx*.42,cy:y+dy*.35,opacity:strength*.5});}
};
