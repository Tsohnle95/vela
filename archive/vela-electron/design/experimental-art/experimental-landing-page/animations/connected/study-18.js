export default {
 name:'Inset bevel',family:'Geometry',description:'An inset contour emerges as the pearl nears each cube, suggesting a polished bevel.',rule:'Contour depth responds to proximity, with a slight offset toward the pearl.',
 setup({path,faceEffects,faces,set}){const nodes=faces.map((d,i)=>path(faceEffects[i],d,'#f2e5ff',3));return ({i,x,y,nx,ny,strength})=>{const scale=.94-strength*.035;set(nodes[i],{transform:`translate(${x+nx*strength*3} ${y+ny*strength*3}) scale(${scale}) translate(${-x} ${-y})`,opacity:strength*.32});};}
};
